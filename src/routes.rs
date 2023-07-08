use axum::{
    debug_handler,
    extract::{Query, State},
    http::{StatusCode, Uri},
    response::{Html, IntoResponse},
};

use color_eyre::eyre::eyre;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, BadError, BottomError, MiddleError, TopError};

#[debug_handler]
pub async fn handler() -> Html<&'static str> {
    Html("<h1>Hello, World!</h1>")
}

#[debug_handler]
#[tracing::instrument]
pub async fn handler_test() -> impl IntoResponse {
    Html("<h1>Hello, Test!</h1>")
}

#[derive(Debug, Deserialize)]
pub struct Person {
    name: String,
}

#[debug_handler]
#[tracing::instrument(fields(name = %person.name))]
pub async fn handler_query(Query(person): Query<Person>) -> impl IntoResponse {
    Html(format!("<h1>Hello, {}!</h1>", &person.name))
}

#[debug_handler]
#[tracing::instrument]
pub async fn fallback(uri: Uri) -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, format!("Cannot find {}", uri.path()))
}

#[debug_handler]
#[tracing::instrument(skip(server_id))]
pub async fn handler_error(State(server_id): State<Uuid>) -> Result<(), ApiError> {
    tracing::info!("Server ID: {}", server_id);
    top_error().map_err(|err| ApiError::UnexpectedError(eyre!(err)))
}

fn top_error() -> Result<(), TopError> {
    middle_error().map_err(|err| TopError::UnexpectedError(eyre!(err)))
}

fn middle_error() -> Result<(), MiddleError> {
    bottom_error().map_err(|err| MiddleError::UnexpectedError(eyre!(err)))
}

fn bottom_error() -> Result<(), BottomError> {
    bare_metal().map_err(|err| BottomError::UnexpectedError(eyre!(err)))
}

fn bare_metal() -> Result<(), BadError> {
    let error = std::io::Error::new(std::io::ErrorKind::Other, "Dinosaurs Mating");
    Err(BadError(error))
}
