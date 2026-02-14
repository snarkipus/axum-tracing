# Observability Notes

## Goals

This crate is trace-first. It focuses on creating high-quality HTTP server spans for Axum and exporting them over OTLP.

## Architecture

1. `init_tracing` configures `tracing_subscriber` and an optional OTLP span exporter.
2. `TelemetryLayer::new(config).build()` builds the Axum/Tower layer stack for request instrumentation.
3. Middleware extracts incoming context, creates request spans, and records response status.

## Span Conventions

The request span includes:

- `otel.name` (e.g. `HTTP GET /query`)
- `otel.kind` (`server`)
- `otel.status_code` (`OK` or `ERROR`)
- `http.method`
- `http.route`
- `http.target`
- `http.status_code`
- `request.id`
- `app.context` (reserved for custom enrichment)

## Context Propagation

- `x-request-id` is generated and propagated.

`traceparent` response propagation can be toggled with `HttpTelemetryConfig::include_trace_response_header`.

## Lifecycle Hooks

`TelemetryLayerBuilder` supports lifecycle customization with:

- `with_request_start_hook` to define span creation
- `with_request_end_hook` to record outcome metadata (`status_code`, `latency`, `is_error`)

If no lifecycle hooks are configured, default HTTP span behavior is used.

## Custom Enrichment Hook

`TelemetryLayerBuilder::with_span_enricher` accepts a closure:

```rust
|context, span| {
    span.record("app.context", format!("{} {}", context.method, context.target));
}
```

Use this hook for domain context that helps debug request behavior.

Compatibility behavior:

- `with_span_enricher` remains supported for migration.
- If hooks and enricher are configured together, hook behavior is primary and
  the enricher runs additively on the created span.

## Handler Extractors

`axum-tracing` provides ergonomic extractors:

- `RootSpan` for span enrichment in handlers
- `RequestId` for request id access without manual extension lookup

Missing telemetry context returns explicit extractor rejections.

## Troubleshooting

- No exported spans:
  - Verify `OTEL_EXPORTER_OTLP_ENDPOINT` is reachable.
  - Confirm the OTLP protocol matches your backend (`grpc` vs `http/protobuf`).
  - Check that `init_tracing` is called before serving requests.
- Missing `x-request-id`:
  - Ensure telemetry layer is mounted on the router handling the request.
