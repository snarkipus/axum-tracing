//! Configuration types for tracing initialization and HTTP telemetry behavior.

use std::env;

/// OTLP transport protocol used when exporting traces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpProtocol {
    /// gRPC OTLP transport (`grpc`).
    Grpc,
    /// HTTP binary OTLP transport (`http/protobuf`).
    HttpBinary,
}

impl OtlpProtocol {
    /// Returns protocol value expected by OTLP-related environment variables.
    pub fn as_str(self) -> &'static str {
        match self {
            OtlpProtocol::Grpc => "grpc",
            OtlpProtocol::HttpBinary => "http/protobuf",
        }
    }
}

/// Runtime tracing/exporter configuration consumed by [`crate::init_tracing`].
///
/// Use [`TracingConfig::from_env`] to apply conventional environment variables
/// on top of the crate defaults.
///
/// # Examples
///
/// ```
/// use axum_tracing::TracingConfig;
///
/// let config = TracingConfig::from_env();
/// assert!(!config.service_name.is_empty());
/// ```
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// Service name emitted in OTEL resource attributes.
    pub service_name: String,
    /// Service version emitted in OTEL resource attributes.
    pub service_version: String,
    /// Deployment environment emitted in OTEL resource attributes.
    pub deployment_environment: String,
    /// Fallback log filter used when `RUST_LOG` is not set.
    pub log_filter: String,
    /// OTLP collector endpoint; set to `None` to disable OTLP export.
    pub otlp_endpoint: Option<String>,
    /// OTLP protocol for trace exporting.
    pub otlp_protocol: OtlpProtocol,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            service_name: "axum-telemetry-layer".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            deployment_environment: "local".to_string(),
            log_filter: "info".to_string(),
            otlp_endpoint: Some("http://localhost:4317".to_string()),
            otlp_protocol: OtlpProtocol::Grpc,
        }
    }
}

impl TracingConfig {
    /// Builds config from defaults and known environment variables.
    ///
    /// Precedence is `environment value -> default value` for every field.
    ///
    /// Variable mapping and fallback behavior:
    /// - `OTEL_SERVICE_NAME` -> [`TracingConfig::service_name`]
    /// - `OTEL_SERVICE_VERSION` -> [`TracingConfig::service_version`]
    /// - `DEPLOYMENT_ENVIRONMENT` -> [`TracingConfig::deployment_environment`]
    /// - `RUST_LOG` -> [`TracingConfig::log_filter`]
    /// - `OTEL_EXPORTER_OTLP_ENDPOINT` -> [`TracingConfig::otlp_endpoint`]
    /// - `OTEL_EXPORTER_OTLP_TRACES_PROTOCOL` -> [`TracingConfig::otlp_protocol`]
    ///
    /// `OTEL_EXPORTER_OTLP_TRACES_PROTOCOL` accepts `grpc` and
    /// `http/protobuf` (case-insensitive). Any other value falls back to
    /// [`OtlpProtocol::Grpc`].
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Some(value) = env_var("OTEL_SERVICE_NAME") {
            config.service_name = value;
        }
        if let Some(value) = env_var("OTEL_SERVICE_VERSION") {
            config.service_version = value;
        }
        if let Some(value) = env_var("DEPLOYMENT_ENVIRONMENT") {
            config.deployment_environment = value;
        }
        if let Some(value) = env_var("RUST_LOG") {
            config.log_filter = value;
        }
        if let Some(value) = env_var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            config.otlp_endpoint = Some(value);
        }
        if let Some(value) = env_var("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL") {
            config.otlp_protocol = parse_protocol(&value);
        }

        config
    }
}

fn env_var(name: &str) -> Option<String> {
    env::var(name).ok()
}

/// HTTP middleware configuration used by [`crate::TelemetryLayer`].
#[derive(Debug, Clone)]
pub struct HttpTelemetryConfig {
    /// Whether incoming `traceparent` should be echoed on responses.
    ///
    /// Default: `true`.
    pub include_trace_response_header: bool,
    /// Header used for request-id generation and propagation.
    ///
    /// Default: `x-request-id`.
    pub request_id_header: &'static str,
}

impl Default for HttpTelemetryConfig {
    fn default() -> Self {
        Self {
            include_trace_response_header: true,
            request_id_header: "x-request-id",
        }
    }
}

fn parse_protocol(raw: &str) -> OtlpProtocol {
    if raw.eq_ignore_ascii_case("http/protobuf") {
        OtlpProtocol::HttpBinary
    } else {
        OtlpProtocol::Grpc
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_protocol, OtlpProtocol};

    #[test]
    fn protocol_parser_prefers_grpc_by_default() {
        assert_eq!(parse_protocol("something-else"), OtlpProtocol::Grpc);
    }

    #[test]
    fn protocol_parser_supports_http_binary() {
        assert_eq!(parse_protocol("http/protobuf"), OtlpProtocol::HttpBinary);
    }
}
