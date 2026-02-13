mod fixtures;

use axum::body::Body;
use tower::ServiceExt;

fn request(path: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("request must be valid")
}

#[tokio::test]
async fn response_contains_request_id() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    assert!(response.headers().contains_key("x-request-id"));
}

#[tokio::test]
async fn request_id_is_propagated_when_provided() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(
            axum::http::Request::builder()
                .uri("/")
                .header("x-request-id", "external-request-id")
                .body(Body::empty())
                .expect("request must be valid"),
        )
        .await
        .expect("request should succeed");

    assert_eq!(
        response
            .headers()
            .get("x-request-id")
            .expect("header exists"),
        "external-request-id"
    );
}
