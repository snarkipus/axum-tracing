use std::env;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OtlpProtocol {
    Grpc,
    HttpBinary,
}

impl OtlpProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            OtlpProtocol::Grpc => "grpc",
            OtlpProtocol::HttpBinary => "http/protobuf",
        }
    }
}

#[derive(Debug, Clone)]
pub struct TracingConfig {
    pub service_name: String,
    pub service_version: String,
    pub deployment_environment: String,
    pub log_filter: String,
    pub otlp_endpoint: Option<String>,
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
    pub fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(value) = env::var("OTEL_SERVICE_NAME") {
            config.service_name = value;
        }
        if let Ok(value) = env::var("OTEL_SERVICE_VERSION") {
            config.service_version = value;
        }
        if let Ok(value) = env::var("DEPLOYMENT_ENVIRONMENT") {
            config.deployment_environment = value;
        }
        if let Ok(value) = env::var("RUST_LOG") {
            config.log_filter = value;
        }
        if let Ok(value) = env::var("OTEL_EXPORTER_OTLP_ENDPOINT") {
            config.otlp_endpoint = Some(value);
        }
        if let Ok(value) = env::var("OTEL_EXPORTER_OTLP_TRACES_PROTOCOL") {
            config.otlp_protocol = parse_protocol(&value);
        }

        config
    }
}

#[derive(Debug, Clone)]
pub struct HttpTelemetryConfig {
    pub include_trace_response_header: bool,
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
