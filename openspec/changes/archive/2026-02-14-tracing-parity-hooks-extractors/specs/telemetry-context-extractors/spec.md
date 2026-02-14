## ADDED Requirements

### Requirement: RootSpan extractor for handlers
The crate MUST provide a `RootSpan` extractor for Axum handlers so handlers can access and enrich the request root span ergonomically.

#### Scenario: Handler records authenticated context
- **WHEN** a handler receives `RootSpan` for a request after middleware has created the root span
- **THEN** the handler can record additional domain fields and events on that root span

### Requirement: RequestId extractor for handlers
The crate MUST provide a `RequestId` extractor for Axum handlers so handlers can access request identity without manual extension lookups.

#### Scenario: Handler reads request id
- **WHEN** a handler receives `RequestId` during request handling
- **THEN** the handler can read and use request id data without direct `Extensions` access

### Requirement: Deterministic extractor behavior without telemetry context
Extractor behavior MUST be deterministic and documented when required telemetry context is unavailable.

#### Scenario: Missing context returns defined rejection
- **WHEN** `RootSpan` or `RequestId` extraction is attempted and required context is missing
- **THEN** extraction fails with an explicit, documented rejection outcome

### Requirement: Extractor rejection types are descriptive and lightweight
Extractor rejection types MUST provide descriptive error context for operators while remaining lightweight for hot request paths.

#### Scenario: Rejection type contract is explicit
- **WHEN** an extractor rejection is returned
- **THEN** the rejection type is documented, avoids unnecessary allocation-heavy payloads, and conveys actionable failure context
