use axum::{body::Body, routing::get, Router};
use axum_tracing::{
    HttpTelemetryConfig, RequestOutcome, RequestSpanContext, RouterTelemetryExt, TelemetryLayer,
};
use std::sync::{Arc, Mutex};
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

#[tokio::test]
async fn with_telemetry_layer_copies_traceparent_when_enabled() {
    let traceparent = "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01";
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry_layer(TelemetryLayer::default().include_trace_response_header(true));

    let response = app
        .oneshot(request_with_traceparent("/"))
        .await
        .expect("request should succeed");

    assert_eq!(
        response
            .headers()
            .get("traceparent")
            .expect("traceparent header should be copied"),
        traceparent
    );
}

#[tokio::test]
async fn with_telemetry_layer_uses_custom_request_id_header() {
    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry_layer(TelemetryLayer::default().with_request_id_header("x-correlation-id"));

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .header("x-correlation-id", "external-correlation-id")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response
            .headers()
            .get("x-correlation-id")
            .expect("header exists"),
        "external-correlation-id"
    );
    assert!(!response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn with_request_start_hook_receives_request_context() {
    let seen_context = Arc::new(Mutex::new(None::<RequestSpanContext>));
    let seen_context_for_hook = Arc::clone(&seen_context);

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry_layer(
            TelemetryLayer::default().with_request_start_hook(move |context| {
                *seen_context_for_hook
                    .lock()
                    .expect("context lock must not be poisoned") = Some(context.clone());

                tracing::info_span!(
                    "http.request",
                    http.method = %context.method,
                    http.route = %context.route,
                    http.target = %context.target,
                )
            }),
        );

    let _ = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    let captured = seen_context
        .lock()
        .expect("context lock must not be poisoned")
        .clone()
        .expect("start hook should capture context");

    assert_eq!(captured.method, "GET");
    assert_eq!(captured.route, "/");
    assert_eq!(captured.target, "/");
    assert!(captured.request_id.is_some());
}

#[tokio::test]
async fn with_request_end_hook_receives_outcome_context() {
    let seen_outcome = Arc::new(Mutex::new(None::<RequestOutcome>));
    let seen_outcome_for_hook = Arc::clone(&seen_outcome);

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry_layer(TelemetryLayer::default().with_request_end_hook(
            move |_span, outcome| {
                *seen_outcome_for_hook
                    .lock()
                    .expect("outcome lock must not be poisoned") = Some(*outcome);
            },
        ));

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    let captured = seen_outcome
        .lock()
        .expect("outcome lock must not be poisoned")
        .expect("end hook should capture outcome");

    assert_eq!(captured.status_code, response.status().as_u16());
    assert!(!captured.is_error);
}

#[tokio::test]
async fn lifecycle_hooks_take_precedence_and_enricher_is_additive() {
    let events = Arc::new(Mutex::new(Vec::<&'static str>::new()));
    let start_events = Arc::clone(&events);
    let enricher_events = Arc::clone(&events);
    let end_events = Arc::clone(&events);

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry_layer(
            TelemetryLayer::default()
                .with_request_start_hook(move |_context| {
                    start_events
                        .lock()
                        .expect("events lock must not be poisoned")
                        .push("start");
                    tracing::info_span!("http.request")
                })
                .with_span_enricher(move |_context, _span| {
                    enricher_events
                        .lock()
                        .expect("events lock must not be poisoned")
                        .push("enricher");
                })
                .with_request_end_hook(move |_span, _outcome| {
                    end_events
                        .lock()
                        .expect("events lock must not be poisoned")
                        .push("end");
                }),
        );

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");
    assert_eq!(response.status(), axum::http::StatusCode::OK);

    let events = events.lock().expect("events lock must not be poisoned");
    assert_eq!(&*events, &["start", "enricher", "end"]);
}
