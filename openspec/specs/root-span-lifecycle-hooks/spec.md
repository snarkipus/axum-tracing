# root-span-lifecycle-hooks Specification

## Purpose
TBD - created by archiving change tracing-parity-hooks-extractors. Update Purpose after archive.
## Requirements
### Requirement: Request start hook customization
The telemetry layer MUST allow callers to customize root span creation at request start using a reusable API abstraction.

#### Scenario: Custom start hook defines span metadata
- **WHEN** a caller configures a custom request-start hook and an HTTP request is received
- **THEN** the root span is created from the custom hook output, including caller-defined name and initial fields

### Requirement: Request end hook customization with outcome context
The telemetry layer MUST allow callers to customize root span finalization at request end with access to request outcome context, including completion timing.

#### Scenario: End hook handles successful outcome
- **WHEN** a request completes successfully and a custom request-end hook is configured
- **THEN** the hook receives span and success outcome context sufficient to record status, completion metadata, and elapsed duration

#### Scenario: End hook handles error outcome
- **WHEN** request processing ends with an error and a custom request-end hook is configured
- **THEN** the hook receives span and error outcome context sufficient to record error semantics and elapsed duration on the span

### Requirement: Default root span behavior remains available
The telemetry layer MUST provide a default root span strategy when no custom lifecycle hooks are configured.

#### Scenario: Default behavior without custom hooks
- **WHEN** telemetry is enabled without custom start or end hooks
- **THEN** the layer emits root spans using built-in default behavior compatible with current baseline telemetry

### Requirement: Existing span enricher compatibility during migration
The existing closure-based span enrichment API MUST remain usable while lifecycle hooks are introduced.

#### Scenario: Legacy enricher continues to work
- **WHEN** callers configure the legacy span enricher without lifecycle hooks
- **THEN** span enrichment behavior remains functional and backward compatible

### Requirement: Configuration precedence for legacy and hook-based customization
The telemetry layer MUST apply lifecycle hooks as the primary customization path when legacy span enricher and lifecycle hooks are configured at the same time.

#### Scenario: Both legacy enricher and lifecycle hooks are configured
- **WHEN** callers configure lifecycle hooks and a legacy span enricher together
- **THEN** request-start behavior is driven by the lifecycle hook output and legacy enricher runs additively on the resulting span
- **THEN** request-end behavior is driven by the lifecycle end hook

