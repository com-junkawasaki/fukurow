//! HTTP server implementation

use axum::{serve, Router};
use std::net::SocketAddr;
use std::time::Instant;
use tokio::net::TcpListener;
use tracing::{info, error};

use crate::{routes::create_router, handlers::AppState};
use reasoner_core::ReasonerEngine;
use rules_cyber::threat_intelligence::ThreatProcessor;

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub max_connections: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 3000,
            max_connections: 100,
        }
    }
}

/// Reasoner API server
pub struct ReasonerServer {
    config: ServerConfig,
    app_state: AppState,
}

impl ReasonerServer {
    /// Create new server with default configuration
    pub fn new() -> Self {
        Self::with_config(ServerConfig::default())
    }

    /// Create new server with custom configuration
    pub fn with_config(config: ServerConfig) -> Self {
        let reasoner = ReasonerEngine::new();
        let threat_processor = ThreatProcessor::new();

        // Initialize reasoner with default cyber security rules
        {
            let mut rule_engine = reasoner_core::rules::RuleEngine::new();
            rule_engine.add_default_cyber_rules();
            // Note: Adding rules to reasoner would require mutable access
        }

        let app_state = AppState {
            reasoner: std::sync::Arc::new(reasoner),
            threat_processor: std::sync::Arc::new(tokio::sync::RwLock::new(threat_processor)),
            start_time: Instant::now(),
        };

        Self { config, app_state }
    }

    /// Get the server address
    pub fn address(&self) -> SocketAddr {
        format!("{}:{}", self.config.host, self.config.port)
            .parse()
            .expect("Invalid server address")
    }

    /// Create the application router
    pub fn create_app(&self) -> Router {
        create_router(self.app_state.clone())
    }

    /// Start the server
    pub async fn serve(self) -> Result<(), Box<dyn std::error::Error>> {
        let addr = self.address();
        let app = self.create_app();

        info!("Starting Reasoner API server on {}", addr);

        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on {}", addr);

        serve(listener, app)
            .await
            .map_err(|e| {
                error!("Server error: {}", e);
                e.into()
            })
    }

    /// Run the server with graceful shutdown
    pub async fn run_with_shutdown(self, shutdown_signal: impl std::future::Future<Output = ()>) -> Result<(), Box<dyn std::error::Error>> {
        let addr = self.address();
        let app = self.create_app();

        info!("Starting Reasoner API server on {} with graceful shutdown", addr);

        let listener = TcpListener::bind(addr).await?;
        info!("Server listening on {}", addr);

        let server = serve(listener, app);
        let graceful = server.with_graceful_shutdown(shutdown_signal);

        graceful.await.map_err(|e| {
            error!("Server error: {}", e);
            e.into()
        })
    }
}

impl Default for ReasonerServer {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a server with custom reasoner engine
pub fn create_server_with_reasoner(reasoner: ReasonerEngine, config: ServerConfig) -> ReasonerServer {
    let threat_processor = ThreatProcessor::new();

    let app_state = AppState {
        reasoner: std::sync::Arc::new(reasoner),
        threat_processor: std::sync::Arc::new(tokio::sync::RwLock::new(threat_processor)),
        start_time: Instant::now(),
    };

    ReasonerServer { config, app_state }
}

/// Utility function to create a shutdown signal
pub async fn shutdown_signal() {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Shutdown signal received, starting graceful shutdown...");
}
