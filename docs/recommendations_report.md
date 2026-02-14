# Recommendations Report: `axum-tracing` Review

> **Goal**: Achieve feature parity and ergonomic alignment with `tracing-actix-web`.

## Executive Summary

The current `axum-tracing` implementation provides a solid foundation for HTTP telemetry using `tower-http` and `opentelemetry`. However, it lacks the high-level ergonomic abstractions that make `tracing-actix-web` popular, specifically around **customizing the root span** and **accessing span context** from within handlers.

To achieve parity, `axum-tracing` needs to move from a configuration-struct approach to a **Trait-based customization** approach.

## Gap Analysis & Code Review

### 1. Root Span Customization

| Feature           | `tracing-actix-web`                  | Current `axum-tracing`   | Recommendation                                                                                                                                                                      |
| :---------------- | :----------------------------------- | :----------------------- | :---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Customization** | `RootSpanBuilder` Trait              | `SpanEnricher` (Closure) | **Adopt Trait Pattern**. Closures are limited (cannot easily share state or complex logic). A `RootSpanBuilder` trait allows users to define reusable strategies for span creation. |
| **Hooks**         | `on_request_start`, `on_request_end` | `make_span` (internal)   | **Expose Hooks**. Users need to customize what happens when a request _starts_ (naming, fields) and _ends_ (recording status, errors, duration).                                    |
| **Macros**        | `root_span!`                         | None                     | **Optional**. A macro is nice for syntax sugar but `tracing::info_span!` is sufficient for now.                                                                                     |

### 2. Ergonomic Extractors

| Feature         | `tracing-actix-web`   | Current `axum-tracing`     | Recommendation                                                                                                                                                                               |
| :-------------- | :-------------------- | :------------------------- | :------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Access Span** | `RootSpan` extractor  | Manual `Extensions` lookup | **Add `RootSpan` Extractor**. This is critical for "Zero to Hero" observability. Handlers need an easy way to add fields (e.g., `user_id`) to the root span _after_ authentication involves. |
| **Request ID**  | `RequestId` extractor | Manual `Extensions` lookup | **Add `RequestId` Extractor**. Re-export or wrap `tower_http::request_id::RequestId` to make it discoverable.                                                                                |

### 3. Error Handling

- **Current State**: Relies on `tower-http`'s default logging or disjoint implementation.
- **Gap**: `tracing-actix-web` automatically ensures errors are attached to the span.
- **Recommendation**: Ensure the `RootSpanBuilder::on_request_end` hook receives logging/error context so users can decide how to record errors (e.g., as Span events or Status codes).

### 4. Tests

- **Current Coverage**: Good basic coverage of headers and id propagation.
- **Missing**:
  - Tests that verify _custom_ fields on spans.
  - Tests for extracting the root span in a handler.
  - Tests ensuring the span is properly closed/recorded on error.

## specific Recommendations

### Refactor `TelemetryLayer`

Change `TelemetryLayer` to be generic over a `RootSpanBuilder`.

```rust
// Proposed Interface
pub trait RootSpanBuilder {
    fn on_request_start(request: &Request<Body>) -> Span;
    fn on_request_end(span: &Span, response: &Response<Body>);
}
```

### Add Extractors

Create simple wrapper structs that implement `FromRequestParts` for Axum.

```rust
pub struct RootSpan(pub Span);
pub struct RequestId(pub String);
```

### Documentation

Adopt a "Guide-level" documentation style similar to `tracing-actix-web`, explaining _why_ and _how_ to use the customization features, rather than just API docs.

## Detailed Refactoring Impact

### 1. Functionality to Remove

We propose **removing** the closure-based customization in favor of the Trait-based approach.

- **`SpanEnricher` (Type Alias)**:
  - _Current_: `pub type SpanEnricher = Arc<dyn Fn(&RequestSpanContext, &Span) ...>;`
  - _Action_: **Remove**. The `RootSpanBuilder` trait replaces this logic entirely.
- **`TelemetryLayer::with_span_enricher` (Method)**:
  - _Current_: Registers a callback to add attributes to the span.
  - _Action_: **Remove**. Replaced by `TelemetryLayer::new(builder)`.
- **`RequestSpanContext` (Struct)**:
  - _Current_: Passed to the enricher.
  - _Action_: **Refactor/Remove**. The `RootSpanBuilder::on_request_start` will receive the raw `Request` (or parts of it), giving the user full access to 100% of the request data, not just the 4 fields currently in `RequestSpanContext`.

### 2. Functionality to Refactor

- **`HttpSpanMaker` (Internal Struct)**:
  - _Current_: Hardcoded logic to create the span.
  - _Action_: **Refactor** into `DefaultRootSpanBuilder`. This struct will implement the new `RootSpanBuilder` trait and contain the default logic (extracting method, route, etc.).
  - _Benefit_: Users can wrap or compose this default builder if they only want to _add_ headers.
- **`TelemetryLayer` (Struct)**:
  - _Current_: Holds `HttpTelemetryConfig` and `Option<SpanEnricher>`.
  - _Action_: **Genericize**. `pub struct TelemetryLayer<B: RootSpanBuilder>`. It will hold the builder instance `B` instead of the enricher closure.

## Conclusion

The codebase is clean but too rigid. By introducing the `RootSpanBuilder` trait and the `RootSpan` extractor, you will unlock the primary power-user features of `tracing-actix-web` without rewriting the core logic (which currently leverages `tower-http` correctly).
