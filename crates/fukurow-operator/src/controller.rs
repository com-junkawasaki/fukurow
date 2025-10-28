//! # Kubernetes Controller
//!
//! Controller logic for managing FukurowCluster resources

use crate::crds::{FukurowCluster, FukurowClusterStatus, ClusterPhase, ClusterCondition};
use crate::reconciler::FukurowReconciler;
use futures::{StreamExt, TryStreamExt};
use kube::api::{Api, ResourceExt};
use kube::runtime::controller::{Action, Controller};
use kube::runtime::{watcher, WatchStreamExt};
use kube::{Client, CustomResourceExt};
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info, warn};

/// Controller for FukurowCluster resources
pub struct FukurowController {
    client: Client,
    reconciler: Arc<FukurowReconciler>,
}

impl FukurowController {
    /// Create a new controller instance
    pub fn new(client: Client, reconciler: Arc<FukurowReconciler>) -> Self {
        Self { client, reconciler }
    }

    /// Run the controller
    pub async fn run(self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Starting Fukurow controller");

        // Create API for FukurowCluster
        let api: Api<FukurowCluster> = Api::all(self.client.clone());

        // Install CRD if it doesn't exist
        self.install_crd().await?;

        // Create the controller
        let controller = Controller::new(api, watcher::Config::default())
            .run(
                |cluster, _ctx| {
                    let reconciler = Arc::clone(&self.reconciler);
                    async move {
                        match reconciler.reconcile(cluster).await {
                            Ok(action) => action,
                            Err(e) => {
                                error!("Reconciliation failed: {}", e);
                                Action::requeue(Duration::from_secs(30))
                            }
                        }
                    }
                },
                |_cluster, _err, _ctx| {
                    warn!("Reconciliation error: {}", _err);
                    Action::requeue(Duration::from_secs(30))
                },
                watcher::Config::default(),
            )
            .for_each(|_| futures::future::ready(()));

        info!("Controller started successfully");
        controller.await;
        Ok(())
    }

    /// Install the CRD if it doesn't exist
    async fn install_crd(&self) -> Result<(), Box<dyn std::error::Error>> {
        let crd = FukurowCluster::crd();
        let crds: Api<k8s_openapi::apiextensions_apiserver::pkg::apis::apiextensions::v1::CustomResourceDefinition> =
            Api::all(self.client.clone());

        match crds.get(&crd.metadata.name).await {
            Ok(_) => {
                info!("CRD {} already exists", crd.metadata.name);
            }
            Err(kube::Error::Api(e)) if e.code == 404 => {
                info!("Installing CRD {}", crd.metadata.name);
                crds.create(&Default::default(), &crd).await?;
                info!("CRD {} installed successfully", crd.metadata.name);
            }
            Err(e) => {
                return Err(Box::new(e));
            }
        }

        Ok(())
    }

    /// Get controller metrics
    pub fn metrics(&self) -> ControllerMetrics {
        // TODO: Implement metrics collection
        ControllerMetrics {
            reconciliations_total: 0,
            reconciliations_failed: 0,
            reconciliations_duration: Duration::from_secs(0),
        }
    }
}

/// Controller metrics
#[derive(Debug, Clone)]
pub struct ControllerMetrics {
    pub reconciliations_total: u64,
    pub reconciliations_failed: u64,
    pub reconciliations_duration: Duration,
}

impl Default for ControllerMetrics {
    fn default() -> Self {
        Self {
            reconciliations_total: 0,
            reconciliations_failed: 0,
            reconciliations_duration: Duration::from_secs(0),
        }
    }
}

/// Controller configuration
#[derive(Debug, Clone)]
pub struct ControllerConfig {
    /// Requeue interval for failed reconciliations
    pub requeue_interval: Duration,

    /// Maximum number of concurrent reconciliations
    pub max_concurrent_reconciliations: usize,

    /// Enable leader election
    pub enable_leader_election: bool,

    /// Leader election namespace
    pub leader_election_namespace: String,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            requeue_interval: Duration::from_secs(30),
            max_concurrent_reconciliations: 10,
            enable_leader_election: true,
            leader_election_namespace: "kube-system".to_string(),
        }
    }
}

/// Utility functions for status updates
pub struct StatusUpdater {
    client: Client,
}

impl StatusUpdater {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

    /// Update cluster status
    pub async fn update_status(
        &self,
        cluster: &FukurowCluster,
        phase: ClusterPhase,
        message: Option<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let api: Api<FukurowCluster> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let mut status = cluster.status.clone().unwrap_or_default();
        status.phase = phase.clone();
        status.last_update = Some(chrono::Utc::now().to_rfc3339());

        if let Some(msg) = message {
            let condition = ClusterCondition {
                type_: "Ready".to_string(),
                status: match phase {
                    ClusterPhase::Running => "True".to_string(),
                    ClusterPhase::Failed => "False".to_string(),
                    _ => "Unknown".to_string(),
                },
                last_transition_time: chrono::Utc::now().to_rfc3339(),
                reason: format!("{:?}", phase),
                message: msg,
            };

            // Update or add condition
            if let Some(existing) = status.conditions.iter_mut().find(|c| c.type_ == condition.type_) {
                *existing = condition;
            } else {
                status.conditions.push(condition);
            }
        }

        let mut patch = cluster.clone();
        patch.status = Some(status);

        api.replace_status(
            &cluster.metadata.name,
            &Default::default(),
            serde_json::to_vec(&patch)?,
        )
        .await?;

        info!(
            "Updated status for cluster {}/{} to {:?}",
            cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            cluster.metadata.name,
            phase
        );

        Ok(())
    }

    /// Update replica counts
    pub async fn update_replica_counts(
        &self,
        cluster: &FukurowCluster,
        ready: u32,
        total: u32,
        available: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let api: Api<FukurowCluster> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let mut status = cluster.status.clone().unwrap_or_default();
        status.ready_replicas = ready;
        status.replicas = total;
        status.available_replicas = available;
        status.unavailable_replicas = total.saturating_sub(available);
        status.last_update = Some(chrono::Utc::now().to_rfc3339());

        let mut patch = cluster.clone();
        patch.status = Some(status);

        api.replace_status(
            &cluster.metadata.name,
            &Default::default(),
            serde_json::to_vec(&patch)?,
        )
        .await?;

        info!(
            "Updated replica counts for cluster {}/{}: ready={}, total={}, available={}",
            cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            cluster.metadata.name,
            ready,
            total,
            available
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_controller_config_default() {
        let config = ControllerConfig::default();
        assert_eq!(config.requeue_interval, Duration::from_secs(30));
        assert_eq!(config.max_concurrent_reconciliations, 10);
        assert!(config.enable_leader_election);
    }

    #[test]
    fn test_controller_metrics_default() {
        let metrics = ControllerMetrics::default();
        assert_eq!(metrics.reconciliations_total, 0);
        assert_eq!(metrics.reconciliations_failed, 0);
        assert_eq!(metrics.reconciliations_duration, Duration::from_secs(0));
    }

    #[tokio::test]
    async fn test_crd_generation() {
        let crd = FukurowCluster::crd();
        assert_eq!(crd.metadata.name, "fukurowclusters.fukurow.io");
        assert_eq!(crd.spec.group, "fukurow.io");
        assert_eq!(crd.spec.version, "v1");
        assert_eq!(crd.spec.names.kind, "FukurowCluster");
        assert_eq!(crd.spec.names.plural, "fukurowclusters");
    }
}
