# Recommendations Report: `axum-tracing` Review

> **Goal**: Achieve feature parity and ergonomic alignment with `tracing-actix-web`.

## Executive Summary

The current `axum-tracing` implementation provides a solid foundation for HTTP telemetry using `tower-http` and `opentelemetry`. However, it lacks the high-level ergonomic abstractions that make `tracing-actix-web` popular, specifically around **customizing the root span** and **accessing span context** from within handlers.

To achieve parity, `axum-tracing` needs to expose **lifecycle hooks** (start/end) and **extractors**. While a **trait-based customization** approach is recommended for maintainability and ergonomics, the critical requirement is enabling users to customize span creation and closure behavior.

## Gap Analysis & Code Review

### 1. Root Span Customization

| Feature           | `tracing-actix-web`                  | Current `axum-tracing`   | Recommendation                                                                                                                                                                                           |
| :---------------- | :----------------------------------- | :----------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Customization** | `RootSpanBuilder` trait              | `SpanEnricher` (closure) | **Expose hooks (trait recommended)**. Users need reusable strategies. Traits are a robust choice, but closures/trait objects are also valid as long as they allow full access to request data. |
| **Hooks**         | `on_request_start`, `on_request_end` | `make_span` (internal)   | **Expose Hooks**. Users need to customize what happens when a request _starts_ (naming, fields) and _ends_ (recording status, errors, duration).                                                         |
| **Macros**        | `root_span!`                         | None                     | **Optional**. A macro is nice for syntax sugar but `tracing::info_span!` is sufficient for now.                                                                                                          |

### 2. Ergonomic Extractors

| Feature         | `tracing-actix-web`   | Current `axum-tracing`     | Recommendation                                                                                                                                                                               |
| :-------------- | :-------------------- | :------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Access Span** | `RootSpan` extractor  | Manual `Extensions` lookup | **Add a `RootSpan` extractor**. This is critical for "Zero to Hero" observability. Handlers need an easy way to add fields (e.g., `user_id`) to the root span _after_ authentication runs. |
| **Request ID**  | `RequestId` extractor | Manual `Extensions` lookup | **Add a `RequestId` extractor**. Re-export or wrap `tower_http::request_id::RequestId` to make it discoverable.                                                                              |

### 3. Error Handling

- **Current State**: Relies on `tower-http`'s default logging or a separate implementation.
- **Gap**: `tracing-actix-web` automatically ensures errors are attached to the span.
- **Recommendation**: Ensure the request-end hook receives logging/error context so users can decide how to record errors (e.g., as span events or status codes).

### 4. Tests

- **Current Coverage**: Good basic coverage of headers and request-id propagation.
- **Missing**:
  - Tests that verify _custom_ fields on spans.
  - Tests for extracting the root span in a handler.
  - Tests ensuring the span is properly closed/recorded on error.

## Specific Recommendations

### Refactor `TelemetryLayer` (Recommended Direction)

`TelemetryLayer` should expose request-start and request-end customization via a `RootSpanBuilder`-like abstraction. A trait-based design is recommended for long-term ergonomics and maintainability, but other designs can work if they expose equivalent hooks.

```rust
// Proposed Interface
pub trait RootSpanBuilder {
    fn on_request_start<B>(request: &Request<B>) -> Span;
    fn on_request_end<B, E>(span: &Span, outcome: &Result<Response<B>, E>);
}
```

### Add Extractors

Create simple wrapper structs that implement `FromRequestParts` for Axum.

```rust
pub struct RootSpan(pub Span);
pub struct RequestId(pub String);
```

### Documentation

Adopt a "Guide-level" documentation style similar to `tracing-actix-web`, explaining _why_ and _how_ to use customization features, not just API reference docs.

## Detailed Refactoring Impact

### 1. Functionality to Evolve

We recommend evolving the closure-based customization toward a richer hook model. A trait-based API is a strong candidate, but parity does not require traits specifically.

- **`SpanEnricher` (Type Alias)**:
  - _Current_: `pub type SpanEnricher = Arc<dyn Fn(&RequestSpanContext, &Span) ...>;`
  - _Action_: **Keep short-term for compatibility**. Deprecate only after an equivalent start/end hook API is available.
- **`TelemetryLayer::with_span_enricher` (Method)**:
  - _Current_: Registers a callback to add attributes to the span.
  - _Action_: **Keep short-term**. Later deprecate if replaced by a more complete hook-based customization API.
- **`RequestSpanContext` (Struct)**:
  - _Current_: Passed to the enricher.
  - _Action_: **Expand or replace** with a request-start input that exposes enough context for parity-level customization.

### 2. Functionality to Refactor

- **`HttpSpanMaker` (Internal Struct)**:
  - _Current_: Hardcoded logic to create the span.
  - _Action_: **Refactor** into a default request-span strategy (e.g., `DefaultRootSpanBuilder`) that can be reused and composed.
  - _Benefit_: Users can wrap or compose this default builder if they only want to _add_ custom fields.
- **`TelemetryLayer` (Struct)**:
  - _Current_: Holds `HttpTelemetryConfig` and `Option<SpanEnricher>`.
  - _Action_: **Expose hook plumbing (required for parity)**. Genericizing over `RootSpanBuilder` is recommended, but any API shape is acceptable if it provides equivalent start/end customization and extractor ergonomics.

## Conclusion

The codebase is clean but currently limits how users can customize spans. By exposing **lifecycle hooks** and adding the **RootSpan/RequestId extractors**, you will unlock the primary power-user features of `tracing-actix-web`. Adopting a `RootSpanBuilder` trait is the recommended path to achieve this in a structured, maintainable way.
