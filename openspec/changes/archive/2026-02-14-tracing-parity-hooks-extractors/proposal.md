## Why

`axum-tracing` has strong core telemetry plumbing but lacks two parity-level ergonomics provided by `tracing-actix-web`: request lifecycle customization at both start and end of request handling, and first-class handler access to tracing context (root span and request id).

Today, users can enrich spans through `SpanEnricher`, but cannot fully express request-start/request-end behavior in a reusable, explicit abstraction. Handler-side span and request-id access also relies on manual `Extensions` usage, which is less discoverable and less ergonomic. This change is needed now to align the crate with common Rust web observability expectations and unlock parity-oriented adoption.

## What Changes

- Add explicit request lifecycle customization hooks:
  - request start hook for root span creation and initial fields
  - request end hook for outcome-aware recording (status/error/latency context)
- Add ergonomic Axum extractors:
  - `RootSpan` extractor
  - `RequestId` extractor
- Preserve short-term compatibility with existing closure-based enrichment APIs while introducing the richer hook model.
- Add tests for custom span fields, extractor behavior, and error/outcome span closure behavior.
- Add guide-level documentation showing how and why to use these features.

## Capabilities

### New Capabilities

- `root-span-lifecycle-hooks`: Expose request-start and request-end customization hooks with outcome context for parity-level root span control.
- `telemetry-context-extractors`: Provide ergonomic `RootSpan` and `RequestId` extractors for use inside handlers.

### Modified Capabilities

- None.

## Impact

- Affected modules: `src/layer.rs`, `src/lib.rs`, and related config/init wiring.
- New and updated public API surface for telemetry customization and extractors.
- Additional tests in unit and/or integration coverage for hooks, extractors, and error paths.
- Documentation updates in guide-level docs and README sections that describe telemetry customization.
