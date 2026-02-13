use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::{Protocol, SpanExporter, WithExportConfig};
use opentelemetry_sdk::{trace::SdkTracerProvider, Resource};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use crate::{
    config::{OtlpProtocol, TracingConfig},
    errors::TelemetryInitError,
};

#[derive(Debug)]
pub struct TelemetryGuard {
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.tracer_provider.take() {
            let _ = provider.shutdown();
        }
    }
}

pub fn init_tracing(config: TracingConfig) -> Result<TelemetryGuard, TelemetryInitError> {
    let TracingConfig {
        service_name,
        service_version,
        deployment_environment,
        log_filter,
        otlp_endpoint,
        otlp_protocol,
    } = config;

    let filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(log_filter));
    let fmt_layer = tracing_subscriber::fmt::layer().json();

    let Some(endpoint) = otlp_endpoint else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt_layer)
            .try_init()
            .map_err(|err| TelemetryInitError::Subscriber(err.to_string()))?;

        return Ok(TelemetryGuard {
            tracer_provider: None,
        });
    };

    let exporter = match otlp_protocol {
        OtlpProtocol::Grpc => SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .build(),
        OtlpProtocol::HttpBinary => SpanExporter::builder()
            .with_http()
            .with_protocol(Protocol::HttpBinary)
            .with_endpoint(endpoint)
            .build(),
    }
    .map_err(|err| TelemetryInitError::OtlpExporter(err.to_string()))?;

    let resource = Resource::builder()
        .with_service_name(service_name.clone())
        .with_attributes([
            KeyValue::new("service.version", service_version),
            KeyValue::new("deployment.environment", deployment_environment),
        ])
        .build();

    let tracer_provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = tracer_provider.tracer(service_name);
    global::set_tracer_provider(tracer_provider.clone());

    tracing_subscriber::registry()
        .with(filter)
        .with(fmt_layer)
        .with(tracing_opentelemetry::layer().with_tracer(tracer))
        .try_init()
        .map_err(|err| TelemetryInitError::Subscriber(err.to_string()))?;

    Ok(TelemetryGuard {
        tracer_provider: Some(tracer_provider),
    })
}
