use axum::{response::Html, routing::get, Router};
use axum_tracing::{init_tracing, HttpTelemetryConfig, TelemetryLayerBuilder, TracingConfig};

#[tokio::main]
async fn main() {
    let _guard = init_tracing(TracingConfig::from_env()).expect("failed to initialize tracing");

    let app = Router::new()
        .route("/", get(handler))
        .route("/health", get(health))
        .layer(
            TelemetryLayerBuilder::new(HttpTelemetryConfig::default())
                .with_span_enricher(|context, span| {
                    span.record(
                        "app.context",
                        format!("{} {}", context.method, context.target),
                    );
                })
                .build(),
        );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .expect("failed to bind TCP listener");
    tracing::info!(
        "listening on {}",
        listener.local_addr().expect("local addr")
    );
    axum::serve(listener, app)
        .await
        .expect("server exited with error");
}

async fn handler() -> Html<&'static str> {
    Html("<h1>Telemetry layer demo</h1>")
}

async fn health() -> &'static str {
    "ok"
}
