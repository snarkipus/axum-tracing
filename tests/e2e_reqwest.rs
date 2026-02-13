mod fixtures;

use std::net::SocketAddr;

use axum::Router;
use axum_tracing::HttpTelemetryConfig;
use reqwest::Client;

#[tokio::test]
async fn e2e_request_id_is_generated_and_propagated() {
    let server = TestServer::spawn(fixtures::app::app()).await;
    let client = Client::new();

    let response = client
        .get(server.url("/"))
        .send()
        .await
        .expect("request should succeed");

    assert!(response.headers().get("x-request-id").is_some());
}

#[tokio::test]
async fn e2e_traceparent_header_is_present_when_enabled() {
    let server = TestServer::spawn(fixtures::app::app()).await;
    let client = Client::new();

    let response = client
        .get(server.url("/"))
        .header(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        )
        .send()
        .await
        .expect("request should succeed");

    assert!(response.headers().get("traceparent").is_some());
}

#[tokio::test]
async fn e2e_traceparent_header_is_absent_when_disabled() {
    let config = HttpTelemetryConfig {
        include_trace_response_header: false,
        ..HttpTelemetryConfig::default()
    };
    let server = TestServer::spawn(fixtures::app::app_with_config(config)).await;
    let client = Client::new();

    let response = client
        .get(server.url("/"))
        .header(
            "traceparent",
            "00-4bf92f3577b34da6a3ce929d0e0e4736-00f067aa0ba902b7-01",
        )
        .send()
        .await
        .expect("request should succeed");

    assert!(response.headers().get("traceparent").is_none());
}

struct TestServer {
    address: SocketAddr,
    handle: tokio::task::JoinHandle<()>,
}

impl TestServer {
    async fn spawn(app: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind listener");
        let address = listener.local_addr().expect("local addr");
        let handle = tokio::spawn(async move {
            axum::serve(listener, app)
                .await
                .expect("server should run until aborted");
        });

        Self { address, handle }
    }

    fn url(&self, path: &str) -> String {
        format!("http://{}{}", self.address, path)
    }
}

impl Drop for TestServer {
    fn drop(&mut self) {
        self.handle.abort();
    }
}
