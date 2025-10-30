//! # Kubernetes Operator Manager
//!
//! Main manager for the Fukurow Kubernetes operator

use crate::{Controller, FukurowReconciler, OperatorConfig, StatusUpdater};
use kube::Client;
use std::sync::Arc;
use tokio::signal;
use tokio::sync::broadcast;
use tracing::{error, info, warn};

/// Main operator manager
pub struct OperatorManager {
    config: OperatorConfig,
    client: Client,
    reconciler: Arc<FukurowReconciler>,
    status_updater: StatusUpdater,
    shutdown_tx: broadcast::Sender<()>,
}

impl OperatorManager {
    /// Create a new operator manager
    pub async fn new(config: OperatorConfig) -> Result<Self, Box<dyn std::error::Error>> {
        let client = Client::try_default().await?;
        let reconciler = Arc::new(FukurowReconciler::new(client.clone(), config.clone()));
        let status_updater = StatusUpdater::new(client.clone());
        let (shutdown_tx, _) = broadcast::channel(1);

        Ok(Self {
            config,
            client,
            reconciler,
            status_updater,
            shutdown_tx,
        })
    }

    /// Start the operator
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Fukurow Kubernetes Operator v{}", env!("CARGO_PKG_VERSION"));
        info!("Configuration: {:?}", self.config);

        // Create controller
        let controller = Controller::new(
            self.client.clone(),
            Arc::clone(&self.reconciler),
        );

        // Start health check server
        let health_handle = self.start_health_server();

        // Start metrics server if enabled
        let metrics_handle = if self.config.enable_monitoring {
            Some(self.start_metrics_server())
        } else {
            None
        };

        // Start controller in a separate task
        let controller_handle = tokio::spawn(async move {
            if let Err(e) = controller.run().await {
                error!("Controller failed: {}", e);
            }
        });

        // Wait for shutdown signal
        self.wait_for_shutdown().await;

        info!("Shutdown signal received, stopping operator...");

        // Stop controller
        controller_handle.abort();

        // Stop health server
        if let Some(handle) = health_handle {
            handle.abort();
        }

        // Stop metrics server
        if let Some(handle) = metrics_handle {
            handle.abort();
        }

        info!("Fukurow operator stopped");
        Ok(())
    }

    /// Start health check HTTP server
    fn start_health_server(&self) -> tokio::task::JoinHandle<()> {
        let addr = "0.0.0.0:8080".parse().unwrap();
        info!("Starting health check server on {}", addr);

        tokio::spawn(async move {
            use axum::{routing::get, Router};
            use std::net::SocketAddr;

            let app = Router::new()
                .route("/health", get(|| async { "OK" }))
                .route("/ready", get(|| async { "OK" }));

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        })
    }

    /// Start metrics HTTP server
    fn start_metrics_server(&self) -> tokio::task::JoinHandle<()> {
        let addr = "0.0.0.0:9090".parse().unwrap();
        info!("Starting metrics server on {}", addr);

        tokio::spawn(async move {
            use axum::{routing::get, Router};
            use std::net::SocketAddr;

            let app = Router::new()
                .route("/metrics", get(|| async {
                    "# Fukurow Operator Metrics\n\
                     operator_up 1\n"
                }));

            let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
            axum::serve(listener, app).await.unwrap();
        })
    }

    /// Wait for shutdown signals
    async fn wait_for_shutdown(&self) {
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received SIGINT, shutting down");
            }
            _ = signal::unix::signal(signal::unix::SignalKind::terminate()) => {
                info!("Received SIGTERM, shutting down");
            }
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal from internal component");
            }
        }
    }

    /// Get operator configuration
    pub fn config(&self) -> &OperatorConfig {
        &self.config
    }

    /// Get Kubernetes client
    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Send shutdown signal
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }
}

/// Operator builder for fluent configuration
pub struct OperatorBuilder {
    config: OperatorConfig,
}

impl OperatorBuilder {
    pub fn new() -> Self {
        Self {
            config: OperatorConfig::default(),
        }
    }

    pub fn namespace(mut self, namespace: String) -> Self {
        self.config.namespace = namespace;
        self
    }

    pub fn image_registry(mut self, registry: String) -> Self {
        self.config.image_registry = registry;
        self
    }

    pub fn image_tag(mut self, tag: String) -> Self {
        self.config.image_tag = tag;
        self
    }

    pub fn enable_scaling(mut self, enable: bool) -> Self {
        self.config.enable_scaling = enable;
        self
    }

    pub fn enable_monitoring(mut self, enable: bool) -> Self {
        self.config.enable_monitoring = enable;
        self
    }

    pub fn default_replicas(mut self, replicas: u32) -> Self {
        self.config.default_replicas = replicas;
        self
    }

    pub fn max_replicas(mut self, max: u32) -> Self {
        self.config.max_replicas = max;
        self
    }

    pub fn min_replicas(mut self, min: u32) -> Self {
        self.config.min_replicas = min;
        self
    }

    pub async fn build(self) -> Result<OperatorManager, Box<dyn std::error::Error>> {
        OperatorManager::new(self.config).await
    }
}

impl Default for OperatorBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_builder() {
        let builder = OperatorBuilder::new()
            .namespace("test-ns".to_string())
            .image_registry("test-registry".to_string())
            .default_replicas(5)
            .enable_scaling(false);

        // We can't easily test the build() method without a Kubernetes cluster,
        // but we can verify the configuration is set correctly
        assert_eq!(builder.config.namespace, "test-ns");
        assert_eq!(builder.config.image_registry, "test-registry");
        assert_eq!(builder.config.default_replicas, 5);
        assert!(!builder.config.enable_scaling);
    }

    #[tokio::test]
    async fn test_operator_manager_creation() {
        // This test will fail in CI without a Kubernetes cluster,
        // but we can test the error handling
        let config = OperatorConfig::default();
        let result = OperatorManager::new(config).await;

        // In a real Kubernetes environment, this would succeed
        // In CI without k8s, it will fail with connection error
        match result {
            Ok(_) => {
                // Kubernetes cluster available
                assert!(true);
            }
            Err(e) => {
                // No Kubernetes cluster - this is expected in CI
                assert!(e.to_string().contains("kubeconfig") ||
                       e.to_string().contains("connection") ||
                       e.to_string().contains("cluster"));
            }
        }
    }
}
