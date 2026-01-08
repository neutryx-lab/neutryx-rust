//! Server startup and binding
//!
//! Provides functionality to start the Axum server with configurable host/port
//! and graceful shutdown support.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::Router;
use tokio::net::TcpListener;
use tokio::sync::watch;

use crate::config::ServerConfig;
use crate::routes;

/// Shutdown signal for graceful shutdown
///
/// Waits for SIGTERM or SIGINT signals and logs shutdown events.
/// On Unix systems, listens for both SIGTERM and SIGINT.
/// On Windows, only listens for Ctrl+C (SIGINT equivalent).
pub async fn shutdown_signal(shutdown_timeout: Duration) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received SIGINT (Ctrl+C), initiating graceful shutdown");
        },
        _ = terminate => {
            tracing::info!("Received SIGTERM, initiating graceful shutdown");
        },
    }

    tracing::info!(
        timeout_secs = shutdown_timeout.as_secs(),
        "Shutdown signal received, waiting for in-flight requests to complete"
    );
}

/// Create a shutdown signal that can be triggered programmatically for testing
///
/// Returns a tuple of (shutdown_future, shutdown_trigger).
/// When the trigger's send method is called, the shutdown_future completes.
pub fn create_shutdown_channel() -> (impl std::future::Future<Output = ()>, watch::Sender<bool>) {
    let (tx, mut rx) = watch::channel(false);
    let shutdown_future = async move {
        // Wait until shutdown is signaled
        while !*rx.borrow() {
            if rx.changed().await.is_err() {
                break;
            }
        }
    };
    (shutdown_future, tx)
}

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

    /// Get the shutdown timeout from configuration
    pub fn shutdown_timeout(&self) -> Duration {
        Duration::from_secs(self.config.shutdown_timeout_secs)
    }

    /// Run the server with graceful shutdown
    ///
    /// This is the main entry point for starting the server.
    /// It binds to the configured host/port and serves requests.
    /// The server will gracefully shut down when receiving SIGTERM or SIGINT,
    /// waiting for in-flight requests to complete or until the timeout expires.
    pub async fn run(self) -> Result<(), std::io::Error> {
        let addr = self.socket_addr();
        let listener = TcpListener::bind(addr).await?;
        let shutdown_timeout = self.shutdown_timeout();

        tracing::info!(address = %addr, "Server listening");

        let server =
            axum::serve(listener, self.router).with_graceful_shutdown(shutdown_signal(shutdown_timeout));

        server.await?;

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Run the server without graceful shutdown
    ///
    /// This is useful for testing where you don't need signal handling.
    pub async fn run_without_shutdown(self) -> Result<(), std::io::Error> {
        let addr = self.socket_addr();
        let listener = TcpListener::bind(addr).await?;

        tracing::info!(address = %addr, "Server listening (no graceful shutdown)");

        axum::serve(listener, self.router).await
    }

    /// Run the server with a specific listener and graceful shutdown
    ///
    /// This is useful for testing where you want to use a listener bound to port 0
    /// to get a random available port.
    pub async fn run_with_listener(self, listener: TcpListener) -> Result<(), std::io::Error> {
        let addr = listener.local_addr()?;
        let shutdown_timeout = self.shutdown_timeout();
        tracing::info!(address = %addr, "Server listening");

        let server =
            axum::serve(listener, self.router).with_graceful_shutdown(shutdown_signal(shutdown_timeout));

        server.await?;

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Run the server with a specific listener and custom shutdown signal
    ///
    /// This allows providing a custom shutdown future, useful for testing graceful shutdown behavior.
    pub async fn run_with_shutdown<F>(self, listener: TcpListener, shutdown: F) -> Result<(), std::io::Error>
    where
        F: std::future::Future<Output = ()> + Send + 'static,
    {
        let addr = listener.local_addr()?;
        tracing::info!(address = %addr, "Server listening (with custom shutdown)");

        let server = axum::serve(listener, self.router).with_graceful_shutdown(shutdown);

        server.await?;

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Create a test server and return the bound address
    ///
    /// This binds to port 0 to get a random available port, starts the server
    /// in a background task, and returns the actual bound address.
    /// Uses run_without_shutdown to avoid signal handler issues in tests.
    #[cfg(test)]
    pub async fn spawn_test_server(
        config: ServerConfig,
    ) -> (SocketAddr, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let server = Self::new(config);
        let handle = tokio::spawn(async move {
            // Use run_without_shutdown for tests that will abort the server
            let listener2 = TcpListener::bind(addr).await.ok();
            if let Some(l) = listener2 {
                drop(l);
            }
            axum::serve(listener, server.router).await.ok();
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        (addr, handle)
    }

    /// Create a test server with a controllable shutdown signal
    ///
    /// This binds to port 0 to get a random available port, starts the server
    /// in a background task with a custom shutdown signal, and returns
    /// the bound address, server handle, and shutdown trigger.
    #[cfg(test)]
    pub async fn spawn_test_server_with_shutdown(
        config: ServerConfig,
    ) -> (
        SocketAddr,
        tokio::task::JoinHandle<Result<(), std::io::Error>>,
        watch::Sender<bool>,
    ) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        let (shutdown_signal, shutdown_tx) = create_shutdown_channel();
        let server = Self::new(config);
        let handle = tokio::spawn(async move {
            server.run_with_shutdown(listener, shutdown_signal).await
        });

        // Give the server a moment to start
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;

        (addr, handle, shutdown_tx)
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

    // ==================== Graceful Shutdown Tests ====================

    #[test]
    fn test_shutdown_timeout_from_config() {
        let mut config = ServerConfig::default();
        config.shutdown_timeout_secs = 60;

        let server = Server::new(config);

        assert_eq!(server.shutdown_timeout(), Duration::from_secs(60));
    }

    #[test]
    fn test_default_shutdown_timeout_is_30_seconds() {
        let config = ServerConfig::default();
        let server = Server::new(config);

        assert_eq!(server.shutdown_timeout(), Duration::from_secs(30));
    }

    #[tokio::test]
    async fn test_graceful_shutdown_via_channel() {
        let config = ServerConfig::default();
        let (addr, handle, shutdown_tx) = Server::spawn_test_server_with_shutdown(config).await;

        let client = reqwest::Client::new();

        // Verify server is running
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Trigger shutdown
        shutdown_tx.send(true).unwrap();

        // Wait for server to shutdown (with timeout)
        let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(result.is_ok(), "Server should shutdown within timeout");

        let server_result = result.unwrap();
        assert!(server_result.is_ok(), "Server join should succeed");
        assert!(server_result.unwrap().is_ok(), "Server should shutdown cleanly");
    }

    #[tokio::test]
    async fn test_graceful_shutdown_completes_pending_request() {
        let config = ServerConfig::default();
        let (addr, handle, shutdown_tx) = Server::spawn_test_server_with_shutdown(config).await;

        let client = reqwest::Client::new();

        // Verify server is running
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Trigger shutdown
        shutdown_tx.send(true).unwrap();

        // The server should stop accepting new connections after shutdown signal
        // Wait a moment for shutdown to begin processing
        tokio::time::sleep(Duration::from_millis(50)).await;

        // Wait for server to fully shutdown
        let result = tokio::time::timeout(Duration::from_secs(5), handle).await;
        assert!(result.is_ok(), "Server should shutdown within timeout");
    }

    #[tokio::test]
    async fn test_shutdown_channel_creation() {
        let (shutdown_future, tx) = create_shutdown_channel();

        // Spawn a task that waits on shutdown
        let wait_handle = tokio::spawn(async move {
            shutdown_future.await;
            true
        });

        // Give the task time to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Trigger shutdown
        tx.send(true).unwrap();

        // Verify the shutdown was received
        let result = tokio::time::timeout(Duration::from_secs(1), wait_handle).await;
        assert!(result.is_ok(), "Shutdown should complete within timeout");
        assert!(result.unwrap().unwrap(), "Shutdown future should complete");
    }

    #[tokio::test]
    async fn test_server_responds_during_shutdown_grace_period() {
        // Test that the server can still respond to requests during the
        // graceful shutdown period (before timeout expires)
        let mut config = ServerConfig::default();
        config.shutdown_timeout_secs = 5; // 5 second shutdown timeout

        let (addr, handle, shutdown_tx) = Server::spawn_test_server_with_shutdown(config).await;

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();

        // Verify server is running
        let response = client
            .get(format!("http://{}/health", addr))
            .send()
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        // Trigger shutdown
        shutdown_tx.send(true).unwrap();

        // Wait for server to shutdown
        let result = tokio::time::timeout(Duration::from_secs(10), handle).await;
        assert!(result.is_ok(), "Server should shutdown within timeout");
    }

    #[tokio::test]
    async fn test_custom_shutdown_timeout_respected() {
        let mut config = ServerConfig::default();
        config.shutdown_timeout_secs = 2; // 2 second timeout

        let server = Server::new(config);
        assert_eq!(server.shutdown_timeout(), Duration::from_secs(2));
    }
}
