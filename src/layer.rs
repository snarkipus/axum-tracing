//! Axum HTTP telemetry layer and router extension APIs.

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

/// Callback used to enrich the request span with application-specific fields.
///
/// The callback runs once when the request span is created.
pub type SpanEnricher = Arc<dyn Fn(&RequestSpanContext, &Span) + Send + Sync + 'static>;

/// Request metadata captured when creating an HTTP span.
///
/// Field values are extracted from the incoming request before handler
/// execution. `route` is `"fallback"` when Axum has no matched route.
#[derive(Debug, Clone)]
pub struct RequestSpanContext {
    /// HTTP method, for example `GET`.
    pub method: String,
    /// Matched route template, for example `/users/:id`.
    pub route: String,
    /// Request path and query target, for example `/users/42?expand=true`.
    pub target: String,
    /// Request id value if present/assigned by the request-id middleware.
    pub request_id: Option<String>,
}

/// Builder-style configuration wrapper for the HTTP telemetry middleware stack.
///
/// Use [`TelemetryLayer::default`] for sensible defaults, then chain methods to
/// customize request-id and span behavior.
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
    /// Returns a default builder instance.
    pub fn builder() -> Self {
        Self::default()
    }

    /// Creates a builder from explicit [`HttpTelemetryConfig`].
    pub fn new(config: HttpTelemetryConfig) -> Self {
        Self {
            config,
            span_enricher: None,
        }
    }

    /// Enables or disables echoing incoming `traceparent` on responses.
    ///
    /// Equivalent to mutating
    /// [`HttpTelemetryConfig::include_trace_response_header`].
    pub fn include_trace_response_header(mut self, include: bool) -> Self {
        self.config.include_trace_response_header = include;
        self
    }

    /// Sets the header name used for request-id generation and propagation.
    ///
    /// Equivalent to mutating [`HttpTelemetryConfig::request_id_header`].
    pub fn with_request_id_header(mut self, header: &'static str) -> Self {
        self.config.request_id_header = header;
        self
    }

    /// Registers a callback that can add domain-specific span fields.
    ///
    /// The callback receives a snapshot of request metadata and the created span.
    /// This is the intended hook for recording `app.*` attributes.
    pub fn with_span_enricher<F>(mut self, callback: F) -> Self
    where
        F: Fn(&RequestSpanContext, &Span) + Send + Sync + 'static,
    {
        self.span_enricher = Some(Arc::new(callback));
        self
    }

    /// Builds a `tower::Layer` that can be mounted on an Axum router.
    ///
    /// Layer stack behavior:
    /// - assigns/propagates request ids
    /// - creates OpenTelemetry-compatible HTTP request spans
    /// - optionally echoes incoming `traceparent` on responses
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

/// Backward-compatible alias for [`TelemetryLayer`].
pub type TelemetryLayerBuilder = TelemetryLayer;

/// Extension methods for mounting telemetry on an Axum [`Router`].
pub trait RouterTelemetryExt<S> {
    /// Mounts telemetry with default middleware settings.
    ///
    /// # Examples
    ///
    /// ```
    /// use axum::{routing::get, Router};
    /// use axum_tracing::RouterTelemetryExt;
    ///
    /// let app = Router::<()>::new()
    ///     .route("/", get(|| async { "ok" }))
    ///     .with_telemetry();
    /// # let _ = app;
    /// ```
    fn with_telemetry(self) -> Router<S>;

    /// Mounts telemetry with a caller-provided [`TelemetryLayer`] builder.
    ///
    /// # Examples
    ///
    /// ```
    /// use axum::{routing::get, Router};
    /// use axum_tracing::{RouterTelemetryExt, TelemetryLayer};
    ///
    /// let app = Router::<()>::new().route("/", get(|| async { "ok" })).with_telemetry_layer(
    ///     TelemetryLayer::default()
    ///         .include_trace_response_header(false)
    ///         .with_request_id_header("x-correlation-id"),
    /// );
    /// # let _ = app;
    /// ```
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

#[derive(Clone)]
struct HttpSpanMaker {
    span_enricher: Option<SpanEnricher>,
}

impl<B> MakeSpan<B> for HttpSpanMaker {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let method = request.method().to_string();
        let route = request_route(request);
        let target = request_target(request);
        let request_id = request_id(request);
        let otel_name = format!("HTTP {} {}", method, route);

        let span = tracing::info_span!(
            "http.request",
            otel.name = %otel_name,
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

fn request_route<B>(request: &Request<B>) -> String {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| path.as_str().to_string())
        .unwrap_or_else(|| FALLBACK_ROUTE.to_string())
}

fn request_target<B>(request: &Request<B>) -> String {
    let uri = request.uri();
    uri.path_and_query()
        .map(|path| path.as_str().to_string())
        .unwrap_or_else(|| uri.path().to_string())
}

fn request_id<B>(request: &Request<B>) -> Option<String> {
    request
        .extensions()
        .get::<RequestId>()
        .and_then(|id| id.header_value().to_str().ok())
        .map(str::to_owned)
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
