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
async fn verbose_error_route_includes_details() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(request("/error"))
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
    assert!(body.contains("Route Level Error"));
}

#[tokio::test]
async fn opaque_error_route_hides_details() {
    let app = fixtures::app::app();

    let response = app
        .oneshot(request("/error/opaque"))
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
    assert!(bytes.is_empty());
}
