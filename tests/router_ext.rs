use axum::{body::Body, routing::get, Router};
use axum_tracing::{HttpTelemetryConfig, RouterTelemetryExt, TelemetryLayer};
use tower::ServiceExt;

fn request(path: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("request must be valid")
}

fn request_with_traceparent(path: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(path)
        .header(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        )
        .body(Body::empty())
        .expect("request must be valid")
}

#[tokio::test]
async fn with_telemetry_adds_request_id_behavior() {
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry();

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn with_telemetry_layer_respects_traceparent_setting() {
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry_layer(TelemetryLayer::new(HttpTelemetryConfig {
            include_trace_response_header: false,
            ..HttpTelemetryConfig::default()
        }));

    let response = app
        .oneshot(request_with_traceparent("/"))
        .await
        .expect("request should succeed");

    assert!(!response.headers().contains_key("traceparent"));
}
