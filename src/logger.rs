use axum::{
    body::Body,
    extract::{ConnectInfo, MatchedPath, OriginalUri},
    http::Request,
    response::Response,
    Router,
};
use hyper::{http::HeaderName, Version, body::Bytes, HeaderMap};
use std::{borrow::Cow, net::SocketAddr, time::Duration};
use tower::ServiceBuilder;
use tracing::subscriber::set_global_default;
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::{fmt::MakeWriter, layer::SubscriberExt, EnvFilter, Registry};

use tower_http::{
    request_id::{MakeRequestUuid, PropagateRequestIdLayer, RequestId, SetRequestIdLayer},
    trace::TraceLayer, classify::ServerErrorsFailureClass,
};
use tracing::{Span, Subscriber};

use crate::error::{ApiError, OpaqueApiError};

// region: init telemetry
pub fn get_subscriber<Sink>(
    name: String,
    env_filter: String,
    sink: Sink,
) -> impl Subscriber + Send + Sync
where
    Sink: for<'a> MakeWriter<'a> + Send + Sync + 'static,
{
    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(env_filter));
    let formatting_layer = BunyanFormattingLayer::new(name, sink);

    Registry::default()
        .with(env_filter)
        .with(JsonStorageLayer)
        .with(formatting_layer)
}

pub fn init_subscriber(subscriber: impl Subscriber + Send + Sync) {
    LogTracer::init().expect("Failed to set logger.");
    set_global_default(subscriber).expect("Failed to set subscriber.");
}
// endregion: init telemetry

// region: telemetry middleware
pub async fn add_telemetry(route: Router) -> Router {
    route.layer(
        ServiceBuilder::new()
            .layer(SetRequestIdLayer::new(
                HeaderName::from_static("x-request-id"),
                MakeRequestUuid,
            ))
            .layer(
                TraceLayer::new_for_http()
                    .make_span_with(|request: &hyper::Request<Body>| {
                        let user_agent = request
                            .headers()
                            .get("user-agent")
                            .map(|value| value.to_str().unwrap_or(""))
                            .unwrap_or("");

                        let http_route = request
                            .extensions()
                            .get::<MatchedPath>()
                            .map(MatchedPath::as_str)
                            .unwrap_or("fallback");

                        let http_method = request.method().as_str();

                        let client_ip = request
                            .extensions()
                            .get::<ConnectInfo<SocketAddr>>()
                            .map(|addr| addr.ip().to_string())
                            .unwrap_or_else(|| "".to_string());

                        let scheme = request
                            .extensions()
                            .get::<OriginalUri>()
                            .and_then(|uri| uri.scheme_str())
                            .unwrap_or("");

                        let request_id = request
                            .extensions()
                            .get::<RequestId>()
                            .and_then(|id| id.header_value().to_str().ok())
                            .unwrap_or("");

                        tracing::info_span!(
                            "http request",
                            http.method = %http_method,
                            http.route = %http_route,
                            http.flavor = %http_flavor(request.version()),
                            http.scheme = %scheme,
                            http.client_ip = %client_ip,
                            http.user_agent = %user_agent,
                            http.target = %request.uri().path_and_query().map(|p| p.as_str()).unwrap_or(""),
                            http.status_code = tracing::field::Empty,
                            otel.name = %format!("HTTP {} {}", http_method, http_route),
                            otel.kind = "server",
                            otel.status_code = tracing::field::Empty,
                            request_id = %request_id,
                            exception.message = tracing::field::Empty,
                            exception.details = tracing::field::Empty,
                        )
                    })
                    .on_request(|_request: &Request<_>, _span: &Span| {
                        // nothing to see here ...
                    })
                    .on_response(
                        |response: &Response, _latency: Duration, span: &Span| {
                            let mut display = String::new();
                            let mut debug = String::new();
                            
                            if let Some(response_error) = response.extensions().get::<ApiError>() {
                                // pre-formatting errors is a workaround for https://github.com/tokio-rs/tracing/issues/1565
                                display = format!("{response_error}");
                                debug = format!("{response_error:?}");
                            }

                            if let Some(response_error) = response.extensions().get::<OpaqueApiError>() {
                                // pre-formatting errors is a workaround for https://github.com/tokio-rs/tracing/issues/1565
                                display = format!("{response_error}");
                                debug = format!("{response_error:?}");
                            }

                            // Record the response's status code in the span.
                            span.record("http.status_code", response.status().as_u16());
                            
                            match response.status() {
                                // 2xx is fine!
                                code if code.is_success() => {
                                    span.record("exception.message", "");
                                    span.record("exception.details", "");
                                    span.record("otel.status_code", "OK");
                                }
                                // 4xx is a client error.
                                code if code.is_client_error() => {
                                    span.record("exception.message", display);
                                    span.record("exception.details", debug);
                                    span.record("otel.status_code", "OK");
                                }
                                // 5xx is a server error.
                                code if code.is_server_error() => {
                                    span.record("exception.message", display);
                                    span.record("exception.details", debug);
                                    span.record("otel.status_code", "ERROR");
                                }
                                // Responses with any other code are unexpected, so
                                // we'll mark the span as an error.
                                _ => {
                                    span.record("exception.message", "Unexpected Error");
                                    span.record("exception.details", debug);
                                    span.record("otel.status_code", "ERROR");
                                }
                            }                           
                    }) 
                    .on_body_chunk(|_chunk: &Bytes, _latency: Duration, _span: &Span| {
                        // ...
                    })
                    .on_eos(
                        |_trailers: Option<&HeaderMap>, _stream_duration: Duration, _span: &Span| {
                        // ...
                    })
                    .on_failure(|_error: ServerErrorsFailureClass, _latency: Duration, _span: &Span| {
                        // ...
                    })
            )
            .layer(PropagateRequestIdLayer::new(
                HeaderName::from_static("x-request-id"),
            ))
    )
}
// endregion: telemetry middleware

#[inline]
fn http_flavor(version: Version) -> Cow<'static, str> {
    match version {
        Version::HTTP_09 => "0.9".into(),
        Version::HTTP_10 => "1.0".into(),
        Version::HTTP_11 => "1.1".into(),
        Version::HTTP_2 => "2.0".into(),
        Version::HTTP_3 => "3.0".into(),
        other => format!("{other:?}").into(),
    }
}
