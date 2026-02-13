use std::{
    convert::Infallible,
    future::Future,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    body::Body,
    extract::MatchedPath,
    http::{HeaderName, Request, Response},
    response::IntoResponse,
    routing::Route,
    Router,
};
use axum_tracing_opentelemetry::middleware::OtelAxumLayer;
use tower::{Service, ServiceBuilder};
use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    trace::{MakeSpan, TraceLayer},
};
use tracing::Span;

use crate::config::HttpTelemetryConfig;

const FALLBACK_ROUTE: &str = "fallback";
const TRACEPARENT_HEADER: &str = "traceparent";

pub type SpanEnricher = Arc<dyn Fn(&RequestSpanContext, &Span) + Send + Sync + 'static>;

#[derive(Debug, Clone)]
pub struct RequestSpanContext {
    pub method: String,
    pub route: String,
    pub target: String,
    pub request_id: Option<String>,
}

#[derive(Clone)]
pub struct TelemetryLayer {
    config: HttpTelemetryConfig,
    span_enricher: Option<SpanEnricher>,
}

impl Default for TelemetryLayer {
    fn default() -> Self {
        Self::new(HttpTelemetryConfig::default())
    }
}

impl TelemetryLayer {
    pub fn builder() -> Self {
        Self::default()
    }

    pub fn new(config: HttpTelemetryConfig) -> Self {
        Self {
            config,
            span_enricher: None,
        }
    }

    pub fn include_trace_response_header(mut self, include: bool) -> Self {
        self.config.include_trace_response_header = include;
        self
    }

    pub fn with_span_enricher<F>(mut self, callback: F) -> Self
    where
        F: Fn(&RequestSpanContext, &Span) + Send + Sync + 'static,
    {
        self.span_enricher = Some(Arc::new(callback));
        self
    }

    pub fn build(
        self,
    ) -> impl tower::Layer<
        Route,
        Service = impl Service<
            Request<Body>,
            Response = impl IntoResponse + 'static,
            Error = Infallible,
            Future = impl Send + 'static,
        > + Clone
                      + Send
                      + Sync
                      + 'static,
    > + Clone {
        let Self {
            config,
            span_enricher,
        } = self;

        let request_id_header = HeaderName::from_static(config.request_id_header);
        let make_span = HttpSpanMaker { span_enricher };

        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(
                request_id_header.clone(),
                MakeRequestUuid,
            ))
            .layer(PropagateRequestIdLayer::new(request_id_header))
            .option_layer(
                config
                    .include_trace_response_header
                    .then_some(CopyTraceparentLayer),
            )
            .layer(OtelAxumLayer::default())
            .layer(TraceLayer::new_for_http().make_span_with(make_span))
    }
}

pub type TelemetryLayerBuilder = TelemetryLayer;

pub trait RouterTelemetryExt<S> {
    fn with_telemetry(self) -> Router<S>;
    fn with_telemetry_layer(self, layer: TelemetryLayer) -> Router<S>;
}

impl<S> RouterTelemetryExt<S> for Router<S>
where
    S: Clone + Send + Sync + 'static,
{
    fn with_telemetry(self) -> Router<S> {
        self.layer(TelemetryLayer::default().build())
    }

    fn with_telemetry_layer(self, layer: TelemetryLayer) -> Router<S> {
        self.layer(layer.build())
    }
}

pub fn telemetry_layer(
    config: HttpTelemetryConfig,
) -> impl tower::Layer<
    Route,
    Service = impl Service<
        Request<Body>,
        Response = impl IntoResponse + 'static,
        Error = Infallible,
        Future = impl Send + 'static,
    > + Clone
                  + Send
                  + Sync
                  + 'static,
> + Clone {
    TelemetryLayer::new(config).build()
}

#[derive(Clone)]
struct HttpSpanMaker {
    span_enricher: Option<SpanEnricher>,
}

impl<B> MakeSpan<B> for HttpSpanMaker {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let method = request.method().to_string();
        let route = request
            .extensions()
            .get::<MatchedPath>()
            .map(|path| path.as_str().to_string())
            .unwrap_or_else(|| FALLBACK_ROUTE.to_string());
        let uri = request.uri();
        let target = uri
            .path_and_query()
            .map(|path| path.as_str().to_string())
            .unwrap_or_else(|| uri.path().to_string());
        let request_id = request
            .extensions()
            .get::<RequestId>()
            .and_then(|id| id.header_value().to_str().ok())
            .map(|value| value.to_owned());

        let span = tracing::info_span!(
            "http.request",
            otel.name = %format!("HTTP {} {}", method, route),
            otel.kind = "server",
            otel.status_code = tracing::field::Empty,
            http.method = %method,
            http.route = %route,
            http.target = %target,
            http.status_code = tracing::field::Empty,
            request.id = %request_id.as_deref().unwrap_or(""),
            app.context = tracing::field::Empty,
        );

        let context = RequestSpanContext {
            method,
            route,
            target,
            request_id,
        };

        if let Some(enricher) = &self.span_enricher {
            enricher(&context, &span);
        }

        span
    }
}

#[derive(Clone, Copy)]
struct CopyTraceparentLayer;

impl<S> tower::Layer<S> for CopyTraceparentLayer {
    type Service = CopyTraceparentService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CopyTraceparentService { inner }
    }
}

#[derive(Clone)]
struct CopyTraceparentService<S> {
    inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for CopyTraceparentService<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>> + Send,
    S::Future: Send + 'static,
    S::Error: Send + 'static,
    ReqBody: Send + 'static,
    ResBody: Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let traceparent = request.headers().get(TRACEPARENT_HEADER).cloned();
        let future = self.inner.call(request);

        Box::pin(async move {
            let mut response = future.await?;
            if let Some(value) = traceparent {
                response.headers_mut().insert(TRACEPARENT_HEADER, value);
            }
            Ok(response)
        })
    }
}
