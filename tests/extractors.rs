use axum::{body::Body, routing::get, Router};
use axum_tracing::{RequestId, RootSpan, RouterTelemetryExt};
use http_body_util::BodyExt;
use tower::ServiceExt;

fn request(path: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("request must be valid")
}

#[tokio::test]
async fn extractors_work_with_telemetry_layer() {
    async fn handler(root_span: RootSpan, request_id: RequestId) -> String {
        root_span.as_span().record("app.context", "extractor-test");
        request_id.as_str().to_owned()
    }

    let app = Router::new().route("/", get(handler)).with_telemetry();

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");
    let status = response.status();
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();
    let body = String::from_utf8(bytes.to_vec()).expect("body should be utf-8");

    assert_eq!(
        status,
        axum::http::StatusCode::OK,
        "unexpected body: {body}"
    );
    assert!(!body.is_empty());
}

#[tokio::test]
async fn root_span_extractor_rejects_without_telemetry_layer() {
    let app = Router::new().route("/", get(|_root_span: RootSpan| async { "ok" }));

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();
    let body = String::from_utf8(bytes.to_vec()).expect("body should be utf-8");
    assert!(body.contains("missing request telemetry span"));
}

#[tokio::test]
async fn request_id_extractor_rejects_without_telemetry_layer() {
    let app = Router::new().route("/", get(|_request_id: RequestId| async { "ok" }));

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    assert_eq!(
        response.status(),
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();
    let body = String::from_utf8(bytes.to_vec()).expect("body should be utf-8");
    assert!(body.contains("missing request id"));
}
