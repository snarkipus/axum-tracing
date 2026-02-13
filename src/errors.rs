use thiserror::Error;

#[derive(Debug, Error)]
pub enum TelemetryInitError {
    #[error("failed to configure OTLP exporter: {0}")]
    OtlpExporter(String),
    #[error("failed to install tracing subscriber: {0}")]
    Subscriber(String),
}
