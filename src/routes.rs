use axum::{
    debug_handler,
    extract::{Query, State},
    http::{StatusCode, Uri},
    response::{Html, IntoResponse},
};

use color_eyre::eyre::eyre;
use serde::Deserialize;
use uuid::Uuid;

use crate::error::{ApiError, BottomError, MiddleError, TopError};

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
#[tracing::instrument(skip(_server_id))]
pub async fn handler_error(State(_server_id): State<Uuid>) -> Result<(), ApiError> {
    match top_error() {
        Ok(_) => Ok(()),
        Err(err) => Err(ApiError::UnexpectedError(eyre!(err))),
    }
}

fn bottom_error() -> Result<(), BottomError> {
    let error = std::io::Error::new(std::io::ErrorKind::Other, "Terrible IO Error");
    Err(BottomError::UnexpectedError(error))
}

fn middle_error() -> Result<(), MiddleError> {
    match bottom_error() {
        Ok(_) => Ok(()),
        Err(err) => Err(MiddleError::UnexpectedError(eyre!(err))),
    }
}

fn top_error() -> Result<(), TopError> {
    match middle_error() {
        Ok(_) => Ok(()),
        Err(err) => Err(TopError::UnexpectedError(eyre!(err))),
    }
}
