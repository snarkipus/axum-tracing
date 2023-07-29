use axum::{extract::FromRef, routing::get, Router};
use std::net::{SocketAddr, TcpListener};
use uuid::Uuid;

mod error;
mod logger;
mod routes;

#[derive(Clone, Debug, FromRef)]
struct AppState {
    server_id: Uuid,
}

#[tokio::main]
async fn main() {
    let subscriber = logger::get_subscriber("zero2axum".into(), "info".into(), std::io::stdout);
    logger::init_subscriber(subscriber);
    color_eyre::install().unwrap();

    let state = AppState {
        server_id: Uuid::new_v4(),
    };

    let mut app = Router::new()
        .route("/", get(routes::handler))
        .route("/test", get(routes::handler_test))
        .route("/query", get(routes::handler_query))
        .route("/error", get(routes::handler_error))
        .route("/error/opaque", get(routes::handler_error_opaque))
        .with_state(state)
        .fallback(routes::fallback);

    app = logger::add_telemetry(app).await;

    let listener = TcpListener::bind("127.0.0.1:3000").unwrap();
    tracing::info!("listening on {}", listener.local_addr().unwrap());
    axum::Server::from_tcp(listener)
        .unwrap()
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
        .await
        .unwrap();
}
