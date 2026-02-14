//! Axum HTTP telemetry layer and router extension APIs.

use std::{
    borrow::Cow,
    convert::Infallible,
    future::Future,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};

use axum::{
    body::Body,
    extract::FromRequestParts,
    extract::MatchedPath,
    http::{request::Parts, HeaderName, Request, Response as HttpResponse, StatusCode},
    response::IntoResponse,
    routing::Route,
    Router,
};
use axum_tracing_opentelemetry::middleware::OtelAxumLayer;
use pin_project_lite::pin_project;
use tower::{Service, ServiceBuilder};
use tower_http::{
    request_id::{
        MakeRequestUuid, PropagateRequestIdLayer, RequestId as TowerRequestId, SetRequestIdLayer,
    },
    trace::{MakeSpan, OnResponse, TraceLayer},
};
use tracing::Span;

use crate::config::HttpTelemetryConfig;

const FALLBACK_ROUTE: &str = "fallback";
const TRACEPARENT_HEADER: &str = "traceparent";

/// Callback used to enrich the request span with application-specific fields.
///
/// The callback runs once when the request span is created.
pub type SpanEnricher = Arc<dyn Fn(&RequestSpanContext, &Span) + Send + Sync + 'static>;

/// Callback used to customize root span creation at request start.
pub type RequestStartHook = Arc<dyn Fn(&RequestSpanContext) -> Span + Send + Sync + 'static>;

/// Callback used to record request outcome data at request end.
pub type RequestEndHook = Arc<dyn Fn(&Span, &RequestOutcome) + Send + Sync + 'static>;

/// Axum extractor that exposes the current request root span.
#[derive(Debug, Clone)]
pub struct RootSpan(pub Span);

/// Rejection returned when [`RootSpan`] cannot be extracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RootSpanRejection {
    MissingRootSpan,
}

/// Axum extractor that exposes the request id generated/propagated by middleware.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

/// Rejection returned when [`RequestId`] cannot be extracted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RequestIdRejection {
    MissingRequestId,
    InvalidRequestId,
}

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

/// Request outcome metadata available when a response is produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RequestOutcome {
    /// HTTP status code returned to the client.
    pub status_code: u16,
    /// Time elapsed while processing the request.
    pub latency: Duration,
    /// True when status code is treated as an error outcome.
    pub is_error: bool,
}

impl RootSpan {
    /// Returns the inner tracing span.
    pub fn as_span(&self) -> &Span {
        &self.0
    }
}

impl RequestId {
    /// Returns the request id as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl IntoResponse for RootSpanRejection {
    fn into_response(self) -> axum::response::Response<Body> {
        let body = match self {
            Self::MissingRootSpan => {
                "missing request telemetry span; ensure TelemetryLayer is mounted"
            }
        };
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl IntoResponse for RequestIdRejection {
    fn into_response(self) -> axum::response::Response<Body> {
        let body = match self {
            Self::MissingRequestId => {
                "missing request id; ensure TelemetryLayer request-id middleware is mounted"
            }
            Self::InvalidRequestId => "invalid request id header value",
        };
        (StatusCode::INTERNAL_SERVER_ERROR, body).into_response()
    }
}

impl<S> FromRequestParts<S> for RootSpan
where
    S: Send + Sync,
{
    type Rejection = RootSpanRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let span = Span::current();
        if !span.is_none() {
            return Ok(Self(span));
        }

        if parts.extensions.get::<TowerRequestId>().is_some() {
            return Ok(Self(span));
        }

        Err(RootSpanRejection::MissingRootSpan)
    }
}

impl<S> FromRequestParts<S> for RequestId
where
    S: Send + Sync,
{
    type Rejection = RequestIdRejection;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let tower_request_id = parts
            .extensions
            .get::<TowerRequestId>()
            .ok_or(RequestIdRejection::MissingRequestId)?;

        let value = tower_request_id
            .header_value()
            .to_str()
            .map_err(|_| RequestIdRejection::InvalidRequestId)?;

        Ok(Self(value.to_owned()))
    }
}

/// Builder-style configuration wrapper for the HTTP telemetry middleware stack.
///
/// Use [`TelemetryLayer::default`] for sensible defaults, then chain methods to
/// customize request-id and span behavior.
#[derive(Clone)]
pub struct TelemetryLayer {
    config: HttpTelemetryConfig,
    span_enricher: Option<SpanEnricher>,
    request_start_hook: Option<RequestStartHook>,
    request_end_hook: Option<RequestEndHook>,
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
            request_start_hook: None,
            request_end_hook: None,
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

    /// Registers a callback that customizes request-start span creation.
    ///
    /// When configured, this callback determines the root span shape. If a
    /// span enricher is also configured, the enricher runs additively on the
    /// created span.
    pub fn with_request_start_hook<F>(mut self, callback: F) -> Self
    where
        F: Fn(&RequestSpanContext) -> Span + Send + Sync + 'static,
    {
        self.request_start_hook = Some(Arc::new(callback));
        self
    }

    /// Registers a callback that records request-end outcome metadata.
    ///
    /// The callback is invoked with the root span and response outcome,
    /// including status code and elapsed latency.
    pub fn with_request_end_hook<F>(mut self, callback: F) -> Self
    where
        F: Fn(&Span, &RequestOutcome) + Send + Sync + 'static,
    {
        self.request_end_hook = Some(Arc::new(callback));
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
            request_start_hook,
            request_end_hook,
        } = self;

        let request_id_header = HeaderName::from_static(config.request_id_header);
        let make_span = HttpSpanMaker {
            span_enricher,
            request_start_hook,
        };
        let on_response = HttpOnResponse { request_end_hook };

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
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(make_span)
                    .on_response(on_response),
            )
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
    request_start_hook: Option<RequestStartHook>,
}

impl<B> MakeSpan<B> for HttpSpanMaker {
    fn make_span(&mut self, request: &Request<B>) -> Span {
        let method = request.method().as_str();
        let route = request_route(request);
        let target = request_target(request);
        let request_id = request_id(request);

        if self.request_start_hook.is_none() && self.span_enricher.is_none() {
            return default_request_span(method, route.as_ref(), target, request_id);
        }

        let context = RequestSpanContext {
            method: method.to_owned(),
            route: route.into_owned(),
            target: target.to_owned(),
            request_id: request_id.map(str::to_owned),
        };

        let span = if let Some(start_hook) = &self.request_start_hook {
            start_hook(&context)
        } else {
            default_request_span(
                &context.method,
                &context.route,
                &context.target,
                context.request_id.as_deref(),
            )
        };

        if let Some(enricher) = &self.span_enricher {
            enricher(&context, &span);
        }

        span
    }
}

#[derive(Clone)]
struct HttpOnResponse {
    request_end_hook: Option<RequestEndHook>,
}

impl<B> OnResponse<B> for HttpOnResponse {
    fn on_response(self, response: &HttpResponse<B>, latency: Duration, span: &Span) {
        let outcome = RequestOutcome {
            status_code: response.status().as_u16(),
            latency,
            is_error: response.status().is_server_error(),
        };

        if let Some(end_hook) = &self.request_end_hook {
            end_hook(span, &outcome);
            return;
        }

        default_record_outcome(span, &outcome);
    }
}

fn default_request_span(method: &str, route: &str, target: &str, request_id: Option<&str>) -> Span {
    let otel_name = format_args!("HTTP {} {}", method, route);

    tracing::info_span!(
        "http.request",
        otel.name = %otel_name,
        otel.kind = "server",
        otel.status_code = tracing::field::Empty,
        http.method = %method,
        http.route = %route,
        http.target = %target,
        http.status_code = tracing::field::Empty,
        request.id = %request_id.unwrap_or(""),
        app.context = tracing::field::Empty,
    )
}

fn default_record_outcome(span: &Span, outcome: &RequestOutcome) {
    span.record("http.status_code", outcome.status_code);
    span.record(
        "otel.status_code",
        tracing::field::display(if outcome.is_error { "ERROR" } else { "OK" }),
    );
}

fn request_route<B>(request: &Request<B>) -> Cow<'_, str> {
    request
        .extensions()
        .get::<MatchedPath>()
        .map(|path| Cow::Borrowed(path.as_str()))
        .unwrap_or_else(|| Cow::Borrowed(FALLBACK_ROUTE))
}

fn request_target<B>(request: &Request<B>) -> &str {
    let uri = request.uri();
    uri.path_and_query()
        .map(|path| path.as_str())
        .unwrap_or_else(|| uri.path())
}

fn request_id<B>(request: &Request<B>) -> Option<&str> {
    request
        .extensions()
        .get::<TowerRequestId>()
        .and_then(|id| id.header_value().to_str().ok())
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
    S: Service<Request<ReqBody>, Response = HttpResponse<ResBody>> + Send,
    S::Future: Send,
{
    type Response = HttpResponse<ResBody>;
    type Error = S::Error;
    type Future = CopyTraceparentFuture<S::Future>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        let traceparent = request.headers().get(TRACEPARENT_HEADER).cloned();
        let inner = self.inner.call(request);

        CopyTraceparentFuture { inner, traceparent }
    }
}

pin_project! {
    struct CopyTraceparentFuture<F> {
        #[pin]
        inner: F,
        traceparent: Option<axum::http::HeaderValue>,
    }
}

impl<F, ResBody, E> Future for CopyTraceparentFuture<F>
where
    F: Future<Output = Result<HttpResponse<ResBody>, E>>,
{
    type Output = Result<HttpResponse<ResBody>, E>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut this = self.project();

        match this.inner.as_mut().poll(cx) {
            Poll::Ready(Ok(mut response)) => {
                if let Some(value) = this.traceparent.take() {
                    response.headers_mut().insert(TRACEPARENT_HEADER, value);
                }
                Poll::Ready(Ok(response))
            }
            Poll::Ready(Err(err)) => Poll::Ready(Err(err)),
            Poll::Pending => Poll::Pending,
        }
    }
}
