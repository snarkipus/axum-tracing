use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::get,
    Router,
};
use axum_tracing::{HttpTelemetryConfig, RouterTelemetryExt, TelemetryLayer};
use serde::Deserialize;
use thiserror::Error;

pub fn app() -> Router {
    app_with_config(HttpTelemetryConfig::default())
}

pub fn app_with_config(config: HttpTelemetryConfig) -> Router {
    Router::new()
        .route("/", get(index))
        .route("/query", get(query))
        .route("/error", get(verbose_error))
        .route("/error/opaque", get(opaque_error))
        .fallback(fallback)
        .with_telemetry_layer(TelemetryLayer::new(config))
}

async fn index() -> Html<&'static str> {
    Html("<h1>Telemetry fixture</h1>")
}

#[derive(Debug, Deserialize)]
struct Person {
    name: String,
}

async fn query(Query(person): Query<Person>) -> Html<String> {
    Html(format!("<h1>Hello, {}!</h1>", person.name))
}

async fn fallback(uri: axum::http::Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("Cannot find {}", uri.path()))
}

#[derive(Debug, Error)]
enum ApiError {
    #[error("Route Level Error")]
    Unexpected(#[from] std::io::Error),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let body = format!("{}: {}\n\n{:?}", status, self, self);
        (status, body).into_response()
    }
}

#[derive(Debug, Error)]
enum OpaqueApiError {
    #[error("Route Level Opaque Error")]
    Unexpected(#[from] std::io::Error),
}

impl IntoResponse for OpaqueApiError {
    fn into_response(self) -> Response {
        StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

async fn verbose_error() -> Result<(), ApiError> {
    Err(std::io::Error::other("Dinosaurs Mating"))?
}

async fn opaque_error() -> Result<(), OpaqueApiError> {
    Err(std::io::Error::other("Dinosaurs Mating"))?
}
