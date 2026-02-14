# axum-tracing

`axum-tracing` is a library-first telemetry kit for Axum applications.

It provides:

- a reusable HTTP telemetry `Layer`
- OTLP tracing initialization with local config types
- request-id generation/propagation and structured HTTP request spans
- optional trace-context response header propagation (`traceparent`)
- fixture-driven integration tests for route and error contracts

## Quickstart

```rust
use axum::{Router, routing::get};
use axum_tracing::{init_tracing, RouterTelemetryExt, TracingConfig};

#[tokio::main]
async fn main() {
    let _guard = init_tracing(TracingConfig::from_env()).unwrap();

    let app = Router::new()
        .route("/", get(|| async { "ok" }))
        .with_telemetry();

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
```

Advanced customization (snippet):

```rust
use axum::{Router, routing::get};
use axum_tracing::{HttpTelemetryConfig, RouterTelemetryExt, TelemetryLayer};

let app = Router::new()
    .route("/", get(|| async { "ok" }))
    .with_telemetry_layer(
        TelemetryLayer::new(HttpTelemetryConfig {
            include_trace_response_header: true,
            ..HttpTelemetryConfig::default()
        })
            .with_request_id_header("x-correlation-id")
            .with_request_start_hook(|ctx| {
                tracing::info_span!(
                    "http.request",
                    http.method = %ctx.method,
                    http.route = %ctx.route,
                    http.target = %ctx.target,
                )
            })
            .with_request_end_hook(|span, outcome| {
                span.record("http.status_code", outcome.status_code);
                span.record(
                    "otel.status_code",
                    tracing::field::display(if outcome.is_error { "ERROR" } else { "OK" }),
                );
            })
            .with_span_enricher(|ctx, span| {
                span.record("app.context", format!("{} {}", ctx.method, ctx.target));
            }),
    );
```

`with_span_enricher` remains supported for backward compatibility. If lifecycle
hooks and enricher are configured together, lifecycle hooks are primary and the
enricher runs additively on the start-hook span.

## Environment Variables

`TracingConfig::from_env()` applies `environment -> default` precedence.

- `RUST_LOG` (default: `info`)
- `OTEL_SERVICE_NAME` (default: `axum-telemetry-layer`)
- `OTEL_SERVICE_VERSION` (default: crate version)
- `DEPLOYMENT_ENVIRONMENT` (default: `local`)
- `OTEL_EXPORTER_OTLP_ENDPOINT` (default: `http://localhost:4317`)
- `OTEL_EXPORTER_OTLP_TRACES_PROTOCOL` (`grpc` or `http/protobuf`; unknown values fall back to `grpc`)

## Trace Header Behavior

`HttpTelemetryConfig::include_trace_response_header` controls whether `traceparent`
is echoed back on the HTTP response.

- `true` (default): if the request contains `traceparent`, the same value is added to the response
- `false`: no `traceparent` response header is injected by this layer

This is useful for client-side correlation and debugging across service boundaries.

## Local Trace Backends

### Jaeger (default quickstart)

Run Jaeger all-in-one with OTLP enabled and point `OTEL_EXPORTER_OTLP_ENDPOINT` at `http://localhost:4317`.

### SigNoz (optional richer stack)

Use SigNoz when you want traces plus metrics/logs UI in one local environment.

## Validation

- `cargo test --doc`
- `cargo fmt --all -- --check`
- `cargo clippy --all-targets --all-features -- -D warnings`
- `cargo test`

## Load Testing

Use `./bench_middleware.sh` to run a local middleware-focused throughput check.
The script starts the demo app, warms up `/health`, then runs two `hey` passes
against `/`: one without `traceparent` and one with `traceparent`.

Optional environment variables:

- `TOTAL_REQUESTS` (default: `2000`)
- `CONCURRENCY` (default: `50`)
- `BASE_URL` (default: `http://127.0.0.1:3000`)
- `TRACEPARENT_VALUE` (default: W3C example value)
- `SERVER_LOG` (default: `/tmp/axum-tracing-bench.log`)

README snippets should stay aligned with rustdoc examples validated by `cargo test --doc`.

Single test examples:

- Unit exact: `cargo test config::tests::protocol_parser_supports_http_binary -- --exact`
- Integration exact: `cargo test --test error_contracts opaque_error_route_hides_details -- --exact`
- E2E exact: `cargo test --test e2e_reqwest e2e_traceparent_header_is_present_when_enabled -- --exact`
- Substring match: `cargo test request_id`
