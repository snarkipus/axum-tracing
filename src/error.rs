use axum::{
    body::BoxBody,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::fmt::Debug;
use thiserror::Error;

#[derive(Error)]
pub enum ApiError {
    #[error("Route Level Error")]
    UnexpectedError(#[from] color_eyre::Report),
}

impl Debug for ApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApiError::UnexpectedError(err) => {
                tracing::error!("Unexpected: {}", err);
                error_chain_fmt(err, f)
            }
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response<BoxBody> {
        let status = StatusCode::INTERNAL_SERVER_ERROR;
        let body = format!("{}: {}\n\n{:?}", status, self, self);
        let mut response = (status, body).into_response();

        response.extensions_mut().insert(self);
        response
    }
}

#[derive(Error)]
pub enum TopError {
    #[error("Top Level Error")]
    UnexpectedError(#[from] color_eyre::Report),
}

impl Debug for TopError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TopError::UnexpectedError(err) => {
                tracing::error!("Unexpected: {}", err);
                error_chain_fmt(err, f)
            }
        }
    }
}

#[derive(Error)]
pub enum MiddleError {
    #[error("Mid-Level Error")]
    UnexpectedError(#[from] color_eyre::Report),
}

impl Debug for MiddleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MiddleError::UnexpectedError(err) => {
                tracing::error!("Unexpected: {}", err);
                error_chain_fmt(err, f)
            }
        }
    }
}

#[derive(Error)]
pub enum BottomError {
    #[error(transparent)]
    UnexpectedError(#[from] std::io::Error),
}

impl Debug for BottomError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BottomError::UnexpectedError(err) => {
                tracing::error!("Unexpected: {}", err);
                write!(f, "Unexpected: {}", err)
            }
        }
    }
}

pub fn error_chain_fmt(
    e: &color_eyre::Report,
    f: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    for cause in e.chain() {
        writeln!(f, "Caused by:\n\t{}", cause)?;
    }
    Ok(())
}