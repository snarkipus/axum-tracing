# Observability Notes

## Goals

This crate is trace-first. It focuses on creating high-quality HTTP server spans for Axum and exporting them over OTLP.

## Architecture

1. `init_tracing` configures `tracing_subscriber` and an optional OTLP span exporter.
2. `telemetry_layer` builds an Axum/Tower layer stack for request instrumentation.
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

Note: response `traceparent` propagation is planned as a follow-up enhancement.

## Custom Enrichment Hook

`TelemetryLayerBuilder::with_span_enricher` accepts a closure:

```rust
|context, span| {
    span.record("app.context", format!("{} {}", context.method, context.target));
}
```

Use this hook for domain context that helps debug request behavior.

## Troubleshooting

- No exported spans:
  - Verify `OTEL_EXPORTER_OTLP_ENDPOINT` is reachable.
  - Confirm the OTLP protocol matches your backend (`grpc` vs `http/protobuf`).
  - Check that `init_tracing` is called before serving requests.
- Missing `x-request-id`:
  - Ensure telemetry layer is mounted on the router handling the request.
