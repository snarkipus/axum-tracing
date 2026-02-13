# AGENTS.md

Guidance for autonomous coding agents working in `axum-tracing`.

## Project Snapshot

- Language: Rust (edition 2021).
- App type: library-first Axum telemetry kit with a small demo binary.
- Library entry point: `src/lib.rs`.
- Demo entry point: `src/main.rs`.
- Core modules:
  - `src/config.rs` (trace and HTTP telemetry config)
  - `src/init.rs` (subscriber and OTLP tracer provider setup)
  - `src/layer.rs` (reusable telemetry Layer and span enrichment hook)

## Toolchain and Setup

- Install Rust stable toolchain (`rustup default stable`).
- Ensure `cargo`, `rustfmt`, and `clippy` are available.
- Install components if missing:
  - `rustup component add rustfmt`
  - `rustup component add clippy`

## Build, Run, Lint, and Test Commands

### Build

- Debug build: `cargo build`
- Release build: `cargo build --release`
- Check without producing artifacts: `cargo check`

### Run

- Run app locally: `cargo run`
- Default bind address in code: `127.0.0.1:3000`

### Format

- Format all code: `cargo fmt`
- Check formatting only (CI-friendly): `cargo fmt --all -- --check`

### Lint

- Standard lint pass: `cargo clippy --all-targets --all-features`
- Treat warnings as errors: `cargo clippy --all-targets --all-features -- -D warnings`

### Test

- Run all tests: `cargo test`
- Show test stdout/stderr: `cargo test -- --nocapture`
- Run tests in a specific file target: `cargo test --test <integration_test_name>`
- Run a single unit test by exact name:
  - `cargo test module_path::test_name -- --exact`
- Run tests matching a substring:
  - `cargo test test_name_substring`
- Run a single integration test case by exact name:
  - `cargo test --test <integration_test_name> test_name -- --exact`
- Run end-to-end HTTP tests (real socket + reqwest client):
  - `cargo test --test e2e_reqwest`

### Useful validation sequence before handoff

1. `cargo fmt --all -- --check`
2. `cargo clippy --all-targets --all-features -- -D warnings`
3. `cargo test`

## Repository-Specific Coding Conventions

These conventions are inferred from existing code and should be preserved.

### Imports and module organization

- Prefer grouped `use` statements with nested paths (rustfmt style).
- Keep imports deterministic and minimal; remove unused imports.
- Keep crate-local modules declared and re-exported via `lib.rs`.
- Group imports in this order when practical:
  1. external crates
  2. std
  3. crate-local imports

### Formatting and structure

- Use default `rustfmt` formatting (no custom config currently present).
- Keep functions small and focused by layer (config, init, middleware, tests).
- Prefer explicit line breaks for long builder chains (e.g., Axum router/layers).

### Types and APIs

- Favor concrete return types when simple, and `impl Trait` only when it reduces type noise.
- Use local config types (`TracingConfig`, `HttpTelemetryConfig`) as stable API boundaries.
- Hide middleware type complexity behind `TelemetryLayerBuilder::build` and `telemetry_layer`.
- Prefer `RouterTelemetryExt::with_telemetry()` for the default one-line mounting path.
- Use `RouterTelemetryExt::with_telemetry_layer(...)` for advanced per-router customization.
- Derive traits where helpful (`Debug`, `Deserialize`, `Error`, `Clone`, `PartialEq`, `Eq`).

### Naming conventions

- Follow Rust naming defaults:
  - modules/files: `snake_case`
  - functions: `snake_case`
  - structs/enums/traits: `UpperCamelCase`
  - constants/statics: `UPPER_SNAKE_CASE`
- Name APIs by responsibility (`init_tracing`, `telemetry_layer`, `with_span_enricher`).
- Keep extension-method names ergonomic and intent-based (`with_telemetry`, `with_telemetry_layer`).
- Name error enums by boundary/layer (`TelemetryInitError`, fixture route error types).

### Error handling

- Avoid `unwrap`/`expect` in library logic; propagate errors with context.
- Prefer structured custom errors via `thiserror` (`TelemetryInitError`).
- Keep HTTP response mapping for demo/fixtures in test modules, not core library modules.
- Preserve opaque vs verbose error behavior in tests when validating contracts.

### Tracing and observability

- Record meaningful span fields for request context (route, method, target, request id).
- Keep telemetry field names consistent with existing conventions (`http.*`, `otel.*`, `request.*`, `app.*`).
- Use `with_span_enricher` to add domain context without leaking framework internals to callers.
- `HttpTelemetryConfig::include_trace_response_header` controls `traceparent` response header behavior:
  - when enabled, echo incoming `traceparent` on the response
  - when disabled, do not inject a `traceparent` response header

### Axum patterns

- Register demo routes in `Router::new()` chains in `main.rs` or fixture builders.
- Use typed extractors (`Query`, `Uri`) in fixture routes.
- Keep fallback behavior centralized in fixture builder when testing HTTP behavior.
- Use middleware layering through `tower::ServiceBuilder` in `layer.rs`.

## Testing Guidance for New Code

- Prefer unit tests next to modules using `#[cfg(test)] mod tests`.
- For HTTP behavior, add integration tests under `tests/` when practical.
- Prefer `tests/e2e_reqwest.rs` for final network-level verification of headers and middleware behavior.
- Name tests to describe behavior and expected outcome.
- If asserting error paths, verify both status code and body/visibility contract.
- For tracing-related behavior, prioritize deterministic assertions and avoid timing flakiness.

## Change Scope and Safety

- Keep edits minimal and aligned with existing architecture.
- Do not introduce broad refactors unless explicitly requested.
- Preserve public behavior of existing routes unless task requires a change.
- Keep dependency additions intentional; prefer existing crates already in `Cargo.toml`.

## Documentation and Comments

- Add comments only for non-obvious logic or protocol/telemetry semantics.
- Do not add redundant comments that restate code.
- Keep docs concise and close to the code they describe.

## Cursor and Copilot Rules Check

- Checked for Cursor rules in `.cursor/rules/` and `.cursorrules`: none found.
- Checked for Copilot instructions in `.github/copilot-instructions.md`: none found.
- If these files are added later, treat them as additional constraints and update this file.

## Agent Handoff Checklist

- Code compiles: `cargo check` or `cargo build`.
- Formatting clean: `cargo fmt --all -- --check`.
- Lints clean: `cargo clippy --all-targets --all-features -- -D warnings`.
- Tests pass: `cargo test` (and targeted single-test commands as needed).
- Any behavior change is reflected in tests or clearly documented.

## Landing the Plane (Session Completion)

**When ending a work session**, you MUST complete ALL steps below. Work is NOT complete until `git push` succeeds.

**MANDATORY WORKFLOW:**

1. **File issues for remaining work** - Create issues for anything that needs follow-up
2. **Run quality gates** (if code changed) - Tests, linters, builds
3. **Update issue status** - Close finished work, update in-progress items
4. **PUSH TO REMOTE** - This is MANDATORY:
   ```bash
   git pull --rebase
   bd sync
   git push
   git status  # MUST show "up to date with origin"
   ```
5. **Clean up** - Clear stashes, prune remote branches
6. **Verify** - All changes committed AND pushed
7. **Hand off** - Provide context for next session

**CRITICAL RULES:**
- Work is NOT complete until `git push` succeeds
- NEVER stop before pushing - that leaves work stranded locally
- NEVER say "ready to push when you are" - YOU must push
- If push fails, resolve and retry until it succeeds
