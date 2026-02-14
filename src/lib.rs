//! `axum-tracing` is a library-first telemetry kit for Axum services.
//!
//! The crate focuses on trace-oriented HTTP telemetry:
//! - initialize OpenTelemetry tracing with [`init_tracing`]
//! - attach request telemetry middleware with [`RouterTelemetryExt::with_telemetry`]
//! - customize middleware behavior with [`TelemetryLayer`]
//!
//! Start with [`TracingConfig::from_env`] for initialization defaults and then
//! mount the telemetry layer on your `Router`.
//!
//! # Examples
//!
//! ```
//! use axum::{routing::get, Router};
//! use axum_tracing::RouterTelemetryExt;
//!
//! let app = Router::<()>::new()
//!     .route("/", get(|| async { "ok" }))
//!     .with_telemetry();
//! # let _ = app;
//! ```

pub mod config;
pub mod errors;
pub mod init;
pub mod layer;

/// Configuration types for tracing exporter and HTTP telemetry behavior.
pub use config::{HttpTelemetryConfig, OtlpProtocol, TracingConfig};
/// Tracing initialization entrypoints and shutdown guard.
pub use init::{init_tracing, TelemetryGuard};
/// HTTP telemetry layer types and ergonomic Axum router extension methods.
pub use layer::{RouterTelemetryExt, TelemetryLayer, TelemetryLayerBuilder};
