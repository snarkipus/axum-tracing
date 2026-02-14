## 1. Lifecycle hook API

- [x] 1.1 Introduce request-start and request-end hook abstraction in telemetry layer public API.
- [x] 1.2 Implement default root span strategy for no-customization usage.
- [x] 1.3 Wire request-end hook to receive success and error outcome context.

## 2. Extractors

- [x] 2.1 Implement `RootSpan` Axum extractor and extension wiring.
- [x] 2.2 Implement `RequestId` Axum extractor and extension wiring.
- [x] 2.3 Define and document deterministic rejection behavior when extractor context is missing.
- [x] 2.4 Define custom extractor rejection types that are descriptive for debugging and lightweight for request-path performance.

## 3. Compatibility and migration

- [x] 3.1 Keep legacy `SpanEnricher` customization path functional alongside lifecycle hooks.
- [x] 3.2 Implement and document precedence: lifecycle hooks are primary, enricher runs additively after start-hook span creation.
- [x] 3.3 Add migration notes and staged deprecation guidance for legacy APIs.

## 4. Test coverage

- [x] 4.1 Add tests for custom request-start hook span name and initial fields.
- [x] 4.2 Add tests for request-end hook behavior on success and error outcomes, including elapsed-duration context.
- [x] 4.3 Add tests for `RootSpan` extractor handler usage and enrichment.
- [x] 4.4 Add tests for `RequestId` extractor behavior including missing-context rejection.
- [x] 4.5 Add tests for combined configuration precedence (hooks + legacy enricher) to verify deterministic behavior.

## 5. Documentation

- [x] 5.1 Add guide-level docs for lifecycle customization with practical examples.
- [x] 5.2 Add guide-level docs for `RootSpan` and `RequestId` extractor usage in handlers.
- [x] 5.3 Update README and observability docs to reflect parity features and migration path.

## 6. Validation and release readiness

- [x] 6.1 Run `cargo fmt --all -- --check` and fix formatting issues.
- [x] 6.2 Run `cargo clippy --all-targets --all-features -- -D warnings` and address lint findings.
- [x] 6.3 Run `cargo test` and ensure new parity behaviors are covered and passing.
