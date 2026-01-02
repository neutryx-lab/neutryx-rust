//! Server startup and binding
//!
//! Provides functionality to start the Axum server with configurable host/port.

use std::net::SocketAddr;
use std::sync::Arc;

use axum::Router;
use tokio::net::TcpListener;

use crate::config::ServerConfig;
use crate::routes;

/// Server instance that can be started
pub struct Server {
    /// Server configuration
    config: Arc<ServerConfig>,
    /// The built router
    router: Router,
}

impl Server {
    /// Create a new server instance with the given configuration
    pub fn new(config: ServerConfig) -> Self {
        let config = Arc::new(config);
        let router = routes::build_router(config.clone());

        Self { config, router }
    }

    /// Get the socket address the server will bind to
    pub fn socket_addr(&self) -> SocketAddr {
        format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid socket address")
    }

    /// Get the configuration
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// Run the server
    ///
    /// This is the main entry point for starting the server.
    /// It binds to the configured host/port and serves requests.
    pub async fn run(self) -> Result<(), std::io::Error> {
        let addr = self.socket_addr();
        let listener = TcpListener::bind(addr).await?;

        tracing::info!("Server listening on {}", addr);

        axum::serve(listener, self.router).await
    }

    /// Run the server with a specific listener
    ///
    /// This is useful for testing where you want to use a listener bound to port 0
    /// to get a random available port.
    pub async fn run_with_listener(self, listener: TcpListener) -> Result<(), std::io::Error> {
        let addr = listener.local_addr()?;
        tracing::info!("Server listening on {}", addr);

        axum::serve(listener, self.router).await
    }

    /// Create a test server and return the bound address
    ///
    /// This binds to port 0 to get a random available port, starts the server
    /// in a background task, and returns the actual bound address.
    #[cfg(test)]
    pub async fn spawn_test_server(config: ServerConfig) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = Self::new(config);
        let handle = tokio::spawn(async move {
            server.run_with_listener(listener).await.ok();
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        (addr, handle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reqwest::StatusCode;

    #[test]
    fn test_server_socket_addr() {
        let mut config = ServerConfig::default();
        config.host = "127.0.0.1".to_string();
        config.port = 3000;

        let server = Server::new(config);
        let addr = server.socket_addr();

        assert_eq!(addr.to_string(), "127.0.0.1:3000");
    }

    #[test]
    fn test_server_config_access() {
        let mut config = ServerConfig::default();
        config.port = 9999;

        let server = Server::new(config);

        assert_eq!(server.config().port, 9999);
    }

    #[tokio::test]
    async fn test_server_binds_to_configured_port() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        // Verify the server is listening by making a request
        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        // Clean up
        handle.abort();
    }

    #[tokio::test]
    async fn test_server_health_endpoint() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["status"], "healthy");

        handle.abort();
    }

    #[tokio::test]
    async fn test_server_ready_endpoint() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        let client = reqwest::Client::new();
        let response = client
            .get(format!("http://{}/ready", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body: serde_json::Value = response.json().await.unwrap();
        assert_eq!(body["ready"], true);

        handle.abort();
    }

    #[tokio::test]
    async fn test_server_pricing_stub_endpoints() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        let client = reqwest::Client::new();

        // Test vanilla pricing stub
        let response = client
            .post(format!("http://{}/api/v1/price/vanilla", addr))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        // Test asian pricing stub
        let response = client
            .post(format!("http://{}/api/v1/price/asian", addr))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        handle.abort();
    }

    #[tokio::test]
    async fn test_server_greeks_stub_endpoint() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        let client = reqwest::Client::new();

        let response = client
            .post(format!("http://{}/api/v1/greeks", addr))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        handle.abort();
    }

    #[tokio::test]
    async fn test_server_xva_stub_endpoints() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        let client = reqwest::Client::new();

        // Test portfolio XVA stub
        let response = client
            .post(format!("http://{}/api/v1/xva/portfolio", addr))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        // Test counterparty XVA stub
        let response = client
            .post(format!("http://{}/api/v1/xva/counterparty", addr))
            .header("content-type", "application/json")
            .body("{}")
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);

        handle.abort();
    }

    #[tokio::test]
    async fn test_server_unknown_route_returns_404() {
        let config = ServerConfig::default();
        let (addr, handle) = Server::spawn_test_server(config).await;

        let client = reqwest::Client::new();

        let response = client
            .get(format!("http://{}/unknown/path", addr))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        handle.abort();
    }

    #[tokio::test]
    async fn test_multiple_servers_on_different_ports() {
        let config1 = ServerConfig::default();
        let config2 = ServerConfig::default();

        let (addr1, handle1) = Server::spawn_test_server(config1).await;
        let (addr2, handle2) = Server::spawn_test_server(config2).await;

        // Both servers should be running on different ports
        assert_ne!(addr1.port(), addr2.port());

        let client = reqwest::Client::new();

        // Both should respond to health checks
        let response1 = client
            .get(format!("http://{}/health", addr1))
            .send()
            .await
            .unwrap();
        assert_eq!(response1.status(), StatusCode::OK);

        let response2 = client
            .get(format!("http://{}/health", addr2))
            .send()
            .await
            .unwrap();
        assert_eq!(response2.status(), StatusCode::OK);

        handle1.abort();
        handle2.abort();
    }
}
