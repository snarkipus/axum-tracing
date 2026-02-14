//! Error types surfaced by tracing initialization APIs.

use thiserror::Error;

/// Errors returned when tracing initialization cannot be completed.
#[derive(Debug, Error)]
pub enum TelemetryInitError {
    /// OTLP exporter construction failed for the configured endpoint/protocol.
    #[error("failed to configure OTLP exporter: {0}")]
    OtlpExporter(String),
    /// Subscriber installation failed, usually because one is already set.
    #[error("failed to install tracing subscriber: {0}")]
    Subscriber(String),
}
