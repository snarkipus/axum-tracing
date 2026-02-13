mod fixtures;

use axum::body::Body;
use http_body_util::BodyExt;
use tower::ServiceExt;

fn request(path: &str) -> axum::http::Request<Body> {
    axum::http::Request::builder()
        .uri(path)
        .body(Body::empty())
        .expect("request must be valid")
}

#[tokio::test]
async fn root_route_works() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(request("/"))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
}

#[tokio::test]
async fn query_route_uses_extractor() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(request("/query?name=Jane"))
        .await
        .expect("request should succeed");

    let bytes = response
        .into_body()
        .collect()
        .await
        .expect("body should collect")
        .to_bytes();
    let body = String::from_utf8(bytes.to_vec()).expect("body should be utf-8");
    assert!(body.contains("Hello, Jane"));
}

#[tokio::test]
async fn fallback_returns_not_found() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(request("/missing"))
        .await
        .expect("request should succeed");

    assert_eq!(response.status(), axum::http::StatusCode::NOT_FOUND);
}
