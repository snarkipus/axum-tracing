## Context

`axum-tracing` already emits HTTP telemetry and supports span enrichment via `SpanEnricher`, but it does not expose parity-level lifecycle customization (`on_request_start` and `on_request_end`) or first-class handler extractors (`RootSpan`, `RequestId`).

The change must preserve short-term compatibility for existing users while introducing a clearer, reusable API for lifecycle behavior and handler ergonomics. Public API changes are expected in `src/layer.rs` and `src/lib.rs`, with tests and docs updated alongside implementation.

## Goals / Non-Goals

**Goals:**
- Expose request-start and request-end customization hooks with enough context to record status, errors, and latency metadata.
- Provide ergonomic Axum extractors for root span and request id access in handlers.
- Keep existing closure-based enrichment usable during migration.
- Add deterministic tests for hook behavior, extractor behavior, and error-path span closure.
- Document usage in guide-level docs and README-facing examples.

**Non-Goals:**
- Implementing macro parity (`root_span!`) in this change.
- Removing `SpanEnricher` immediately.
- Broad telemetry refactors outside lifecycle hooks, extractors, and compatibility plumbing.

## Decisions

- Introduce a `RootSpanBuilder`-like lifecycle abstraction that models start/end hooks.
  - Rationale: trait-oriented lifecycle hooks are discoverable and reusable across routers/apps.
  - Alternative considered: closure-only expansion; rejected as less structured for multi-method lifecycle behavior.
- Define parity by capabilities (start/end hooks + extractors), not strictly by trait shape.
  - Rationale: preserves flexibility while ensuring user-visible outcomes match expectations.
- Keep a default root span strategy to preserve one-line ergonomics.
  - Rationale: avoids forcing customization for baseline users.
- Maintain `SpanEnricher` compatibility short-term, with deprecation only after equivalent hook coverage exists.
  - Rationale: reduces migration risk for existing adopters.
- Use deterministic precedence when both lifecycle hooks and `SpanEnricher` are configured: hook-based start/end behavior is primary, and enricher runs additively after start-hook span creation.
  - Rationale: preserves existing enrichment value while preventing ambiguity in core lifecycle behavior.
- Add `RootSpan` and `RequestId` extractors with explicit failure behavior when context is missing.
  - Rationale: handler ergonomics should be first-class and predictable.
- Use explicit extractor rejection when required telemetry context is missing.
  - Rationale: makes failure modes visible and testable instead of silently degrading behavior.

## Risks / Trade-offs

- [API surface growth] -> Mitigation: keep public abstractions minimal and document defaults clearly.
- [Dual-path customization complexity during migration] -> Mitigation: constrain overlap period and document precedence/usage.
- [Extractor rejection surprises] -> Mitigation: define explicit, tested rejection semantics and document them.
- [Inconsistent error-path telemetry] -> Mitigation: pass outcome context into request-end hook and cover success/error cases in tests.

## Migration Plan

1. Introduce lifecycle hook API and default implementation without removing existing enricher APIs.
2. Add extractor types and wiring in middleware/request extensions.
3. Add tests validating legacy enricher compatibility plus new hook/extractor behaviors.
4. Update docs with migration guidance and examples for both old and new paths.
5. After equivalent coverage is proven in a follow-up release, mark legacy APIs as deprecated if desired.

## Open Questions

- None.
