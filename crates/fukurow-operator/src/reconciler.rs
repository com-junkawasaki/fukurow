//! # Kubernetes Reconciler
//!
//! Reconciliation logic for FukurowCluster resources

use crate::crds::{FukurowCluster, FukurowClusterStatus, ClusterPhase, ClusterCondition};
use crate::OperatorConfig;
use k8s_openapi::api::apps::v1::{Deployment, DeploymentSpec, DeploymentStatus};
use k8s_openapi::api::core::v1::{
    ConfigMap, Container, EnvVar, PodSpec, PodTemplateSpec, ResourceRequirements,
    Secret, Service, ServicePort, ServiceSpec, Volume, VolumeMount,
};
use k8s_openapi::api::networking::v1::{Ingress, IngressSpec, IngressTls, HTTPIngressPath, HTTPIngressRuleValue, IngressRule};
use k8s_openapi::apimachinery::pkg::apis::meta::v1::LabelSelector;
use kube::api::{Api, DeleteParams, ListParams, Patch, PatchParams, PostParams, ResourceExt};
use kube::Client;
use kube::runtime::controller::Action;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::time::Duration;
use tracing::{error, info, warn};

/// Reconciler for FukurowCluster resources
pub struct FukurowReconciler {
    client: Client,
    config: OperatorConfig,
}

impl FukurowReconciler {
    pub fn new(client: Client, config: OperatorConfig) -> Self {
        Self { client, config }
    }

    /// Main reconciliation logic
    pub async fn reconcile(&self, cluster: Arc<FukurowCluster>) -> Result<Action, Box<dyn std::error::Error>> {
        info!(
            "Reconciling FukurowCluster {}/{}",
            cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
            cluster.metadata.name
        );

        // Ensure namespace exists
        self.ensure_namespace(&cluster).await?;

        // Reconcile ConfigMap
        self.reconcile_config_map(&cluster).await?;

        // Reconcile Secret
        self.reconcile_secret(&cluster).await?;

        // Reconcile Deployment
        self.reconcile_deployment(&cluster).await?;

        // Reconcile Service
        self.reconcile_service(&cluster).await?;

        // Reconcile Ingress (if configured)
        if let Some(ref ingress_spec) = cluster.spec.network.ingress {
            if ingress_spec.enabled {
                self.reconcile_ingress(&cluster).await?;
            }
        }

        // Update status
        self.update_cluster_status(&cluster).await?;

        // Requeue for periodic reconciliation
        Ok(Action::requeue(Duration::from_secs(300))) // 5 minutes
    }

    /// Ensure namespace exists
    async fn ensure_namespace(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let ns_api: Api<k8s_openapi::api::core::v1::Namespace> = Api::all(self.client.clone());
        let namespace = cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string());

        if namespace == "default" {
            return Ok(()); // default namespace always exists
        }

        match ns_api.get(namespace).await {
            Ok(_) => Ok(()),
            Err(kube::Error::Api(e)) if e.code == 404 => {
                let ns = json!({
                    "apiVersion": "v1",
                    "kind": "Namespace",
                    "metadata": {
                        "name": namespace
                    }
                });

                ns_api.create(&PostParams::default(), &serde_json::from_value(ns)?).await?;
                info!("Created namespace {}", namespace);
                Ok(())
            }
            Err(e) => Err(Box::new(e)),
        }
    }

    /// Reconcile ConfigMap
    async fn reconcile_config_map(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let cm_api: Api<ConfigMap> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let config_data = self.generate_config_data(cluster);
        let cm_name = format!("{}-config", cluster.metadata.name);

        let config_map = ConfigMap {
            metadata: self.metadata_with_labels(cluster, &cm_name, "ConfigMap"),
            data: Some(config_data),
            ..Default::default()
        };

        self.apply_resource(cm_api, &cm_name, config_map).await?;
        Ok(())
    }

    /// Reconcile Secret
    async fn reconcile_secret(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let secret_api: Api<Secret> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let secret_name = format!("{}-secret", cluster.metadata.name);

        // Create basic secret (in production, this should be more sophisticated)
        let secret_data = if cluster.spec.config.security.tls_enabled {
            HashMap::from([
                ("tls.crt".to_string(), "dGVzdCBjZXJ0".to_string()), // base64 encoded "test cert"
                ("tls.key".to_string(), "dGVzdCBrZXk=".to_string()), // base64 encoded "test key"
            ])
        } else {
            HashMap::new()
        };

        if !secret_data.is_empty() {
            let secret = Secret {
                metadata: self.metadata_with_labels(cluster, &secret_name, "Secret"),
                data: Some(secret_data),
                type_: Some("kubernetes.io/tls".to_string()),
                ..Default::default()
            };

            self.apply_resource(secret_api, &secret_name, secret).await?;
        }

        Ok(())
    }

    /// Reconcile Deployment
    async fn reconcile_deployment(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let deploy_api: Api<Deployment> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let deployment = self.create_deployment(cluster);
        let deploy_name = cluster.metadata.name.clone();

        self.apply_resource(deploy_api, &deploy_name, deployment).await?;
        Ok(())
    }

    /// Reconcile Service
    async fn reconcile_service(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let svc_api: Api<Service> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let service = self.create_service(cluster);
        let svc_name = format!("{}-service", cluster.metadata.name);

        self.apply_resource(svc_api, &svc_name, service).await?;
        Ok(())
    }

    /// Reconcile Ingress
    async fn reconcile_ingress(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let ingress_api: Api<Ingress> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let ingress = self.create_ingress(cluster)?;
        let ingress_name = format!("{}-ingress", cluster.metadata.name);

        self.apply_resource(ingress_api, &ingress_name, ingress).await?;
        Ok(())
    }

    /// Update cluster status
    async fn update_cluster_status(&self, cluster: &FukurowCluster) -> Result<(), Box<dyn std::error::Error>> {
        let api: Api<FukurowCluster> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        // Get current deployment status
        let deploy_api: Api<Deployment> = Api::namespaced(
            self.client.clone(),
            &cluster.metadata.namespace.as_ref().unwrap_or(&"default".to_string()),
        );

        let deploy_status = match deploy_api.get_status(&cluster.metadata.name).await {
            Ok(status) => status,
            Err(_) => None,
        };

        let mut status = cluster.status.clone().unwrap_or_default();
        status.phase = if let Some(ref deploy_status) = deploy_status {
            if deploy_status.ready_replicas.unwrap_or(0) > 0 {
                ClusterPhase::Running
            } else {
                ClusterPhase::Creating
            }
        } else {
            ClusterPhase::Creating
        };

        status.ready_replicas = deploy_status.as_ref().and_then(|s| s.ready_replicas).unwrap_or(0);
        status.replicas = deploy_status.as_ref().and_then(|s| s.replicas).unwrap_or(0);
        status.available_replicas = deploy_status.as_ref().and_then(|s| s.available_replicas).unwrap_or(0);
        status.unavailable_replicas = status.replicas.saturating_sub(status.available_replicas);
        status.last_update = Some(chrono::Utc::now().to_rfc3339());
        status.version = Some(cluster.spec.image.tag.clone());

        let mut patch = cluster.clone();
        patch.status = Some(status);

        api.replace_status(
            &cluster.metadata.name,
            &Default::default(),
            serde_json::to_vec(&patch)?,
        )
        .await?;

        Ok(())
    }

    /// Create Deployment
    fn create_deployment(&self, cluster: &FukurowCluster) -> Deployment {
        let labels = self.cluster_labels(cluster);
        let image = format!(
            "{}/{}:{}",
            cluster.spec.image.registry,
            cluster.spec.image.repository,
            cluster.spec.image.tag
        );

        let env_vars = cluster.spec.config.env_vars.iter()
            .map(|(k, v)| EnvVar {
                name: k.clone(),
                value: Some(v.clone()),
                value_from: None,
            })
            .collect::<Vec<_>>();

        let container = Container {
            name: "fukurow".to_string(),
            image: Some(image),
            image_pull_policy: Some(match cluster.spec.image.pull_policy {
                crate::crds::PullPolicy::IfNotPresent => "IfNotPresent".to_string(),
                crate::crds::PullPolicy::Always => "Always".to_string(),
                crate::crds::PullPolicy::Never => "Never".to_string(),
            }),
            ports: Some(vec![k8s_openapi::api::core::v1::ContainerPort {
                container_port: cluster.spec.config.server.port as i32,
                protocol: Some("TCP".to_string()),
                ..Default::default()
            }]),
            env: Some(env_vars),
            resources: Some(self.create_resource_requirements(&cluster.spec.resources)),
            volume_mounts: Some(vec![VolumeMount {
                name: "config".to_string(),
                mount_path: "/app/config".to_string(),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let pod_spec = PodSpec {
            containers: vec![container],
            volumes: Some(vec![Volume {
                name: "config".to_string(),
                config_map: Some(k8s_openapi::api::core::v1::ConfigMapVolumeSource {
                    name: Some(format!("{}-config", cluster.metadata.name)),
                    ..Default::default()
                }),
                ..Default::default()
            }]),
            ..Default::default()
        };

        let pod_template = PodTemplateSpec {
            metadata: Some(self.metadata_with_labels(cluster, "fukurow", "Pod")),
            spec: Some(pod_spec),
        };

        let spec = DeploymentSpec {
            replicas: Some(cluster.spec.replicas as i32),
            selector: LabelSelector {
                match_labels: Some(labels.clone()),
                ..Default::default()
            },
            template: pod_template,
            ..Default::default()
        };

        Deployment {
            metadata: self.metadata_with_labels(cluster, &cluster.metadata.name, "Deployment"),
            spec: Some(spec),
            ..Default::default()
        }
    }

    /// Create Service
    fn create_service(&self, cluster: &FukurowCluster) -> Service {
        let labels = self.cluster_labels(cluster);

        let ports = vec![ServicePort {
            name: Some("http".to_string()),
            port: cluster.spec.config.server.port as i32,
            target_port: Some(k8s_openapi::apimachinery::pkg::util::intstr::IntOrString::Int(cluster.spec.config.server.port as i32)),
            protocol: Some("TCP".to_string()),
            ..Default::default()
        }];

        let spec = ServiceSpec {
            selector: Some(labels),
            ports: Some(ports),
            type_: Some(cluster.spec.network.service_type.clone()),
            ..Default::default()
        };

        Service {
            metadata: self.metadata_with_labels(cluster, &format!("{}-service", cluster.metadata.name), "Service"),
            spec: Some(spec),
            ..Default::default()
        }
    }

    /// Create Ingress
    fn create_ingress(&self, cluster: &FukurowCluster) -> Result<Ingress, Box<dyn std::error::Error>> {
        let ingress_spec = cluster.spec.network.ingress.as_ref()
            .ok_or("Ingress spec is required")?;

        let rules = ingress_spec.hosts.iter()
            .map(|host| IngressRule {
                host: Some(host.clone()),
                http: Some(HTTPIngressRuleValue {
                    paths: vec![HTTPIngressPath {
                        path: Some("/".to_string()),
                        path_type: "Prefix".to_string(),
                        backend: k8s_openapi::api::networking::v1::IngressBackend {
                            service: Some(k8s_openapi::api::networking::v1::IngressServiceBackend {
                                name: format!("{}-service", cluster.metadata.name),
                                port: Some(k8s_openapi::api::networking::v1::ServiceBackendPort {
                                    number: Some(cluster.spec.config.server.port as i32),
                                    ..Default::default()
                                }),
                            }),
                            ..Default::default()
                        },
                    }],
                }),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let tls = ingress_spec.tls.iter()
            .map(|t| IngressTls {
                hosts: Some(t.hosts.clone()),
                secret_name: Some(t.secret_name.clone()),
                ..Default::default()
            })
            .collect::<Vec<_>>();

        let spec = IngressSpec {
            rules: Some(rules),
            tls: if tls.is_empty() { None } else { Some(tls) },
            ..Default::default()
        };

        Ok(Ingress {
            metadata: self.metadata_with_labels(cluster, &format!("{}-ingress", cluster.metadata.name), "Ingress"),
            spec: Some(spec),
            ..Default::default()
        })
    }

    /// Generate configuration data for ConfigMap
    fn generate_config_data(&self, cluster: &FukurowCluster) -> HashMap<String, String> {
        let mut data = HashMap::new();

        // Server config
        data.insert("FUROW_SERVER_PORT".to_string(),
                   cluster.spec.config.server.port.to_string());
        data.insert("FUROW_SERVER_HOST".to_string(),
                   cluster.spec.config.server.host.clone());
        data.insert("FUROW_MAX_CONNECTIONS".to_string(),
                   cluster.spec.config.server.max_connections.to_string());

        // Engine config
        data.insert("FUROW_MAX_CONCURRENT_TASKS".to_string(),
                   cluster.spec.config.engine.max_concurrent_tasks.to_string());
        data.insert("FUROW_REASONING_TIMEOUT".to_string(),
                   cluster.spec.config.engine.reasoning_timeout_seconds.to_string());
        data.insert("FUROW_BATCH_SIZE".to_string(),
                   cluster.spec.config.engine.batch_size.to_string());

        // Security config
        data.insert("FUROW_TLS_ENABLED".to_string(),
                   cluster.spec.config.security.tls_enabled.to_string());

        // Monitoring config
        data.insert("FUROW_METRICS_ENABLED".to_string(),
                   cluster.spec.monitoring.prometheus_enabled.to_string());
        data.insert("FUROW_METRICS_PORT".to_string(),
                   cluster.spec.monitoring.metrics_port.to_string());

        data
    }

    /// Create resource requirements
    fn create_resource_requirements(&self, req: &crate::crds::ResourceRequirements) -> ResourceRequirements {
        let mut requests = std::collections::BTreeMap::new();
        requests.insert("cpu".to_string(), req.requests.cpu.clone());
        requests.insert("memory".to_string(), req.requests.memory.clone());

        let mut limits = std::collections::BTreeMap::new();
        limits.insert("cpu".to_string(), req.limits.cpu.clone());
        limits.insert("memory".to_string(), req.limits.memory.clone());

        ResourceRequirements {
            requests: Some(requests),
            limits: Some(limits),
            ..Default::default()
        }
    }

    /// Create metadata with labels
    fn metadata_with_labels(&self, cluster: &FukurowCluster, name: &str, kind: &str) -> k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
        k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
            name: Some(name.to_string()),
            namespace: cluster.metadata.namespace.clone(),
            labels: Some(self.cluster_labels(cluster)),
            owner_references: Some(vec![k8s_openapi::apimachinery::pkg::apis::meta::v1::OwnerReference {
                api_version: "fukurow.io/v1".to_string(),
                kind: "FukurowCluster".to_string(),
                name: cluster.metadata.name.clone(),
                uid: cluster.metadata.uid.clone().unwrap_or_default(),
                controller: Some(true),
                block_owner_deletion: Some(true),
            }]),
            ..Default::default()
        }
    }

    /// Get cluster labels
    fn cluster_labels(&self, cluster: &FukurowCluster) -> HashMap<String, String> {
        HashMap::from([
            ("app.kubernetes.io/name".to_string(), "fukurow".to_string()),
            ("app.kubernetes.io/instance".to_string(), cluster.metadata.name.clone()),
            ("app.kubernetes.io/version".to_string(), cluster.spec.image.tag.clone()),
            ("app.kubernetes.io/component".to_string(), "reasoning-engine".to_string()),
            ("app.kubernetes.io/part-of".to_string(), "fukurow-cluster".to_string()),
        ])
    }

    /// Apply or update a Kubernetes resource
    async fn apply_resource<T>(
        &self,
        api: Api<T>,
        name: &str,
        resource: T,
    ) -> Result<(), Box<dyn std::error::Error>>
    where
        T: kube::api::Meta + serde::Serialize + Clone,
    {
        match api.get(name).await {
            Ok(_) => {
                // Update existing resource
                api.replace(name, &Default::default(), &resource).await?;
                info!("Updated {} {}", std::any::type_name::<T>(), name);
            }
            Err(kube::Error::Api(e)) if e.code == 404 => {
                // Create new resource
                api.create(&PostParams::default(), &resource).await?;
                info!("Created {} {}", std::any::type_name::<T>(), name);
            }
            Err(e) => return Err(Box::new(e)),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crds::*;

    #[test]
    fn test_generate_config_data() {
        let cluster = FukurowCluster {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-cluster".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: FukurowClusterSpec {
                replicas: 3,
                image: ImageSpec {
                    registry: "ghcr.io/gftdcojp".to_string(),
                    repository: "fukurow".to_string(),
                    tag: "latest".to_string(),
                    pull_policy: PullPolicy::IfNotPresent,
                },
                resources: ResourceRequirements {
                    requests: ResourceQuantity {
                        cpu: "100m".to_string(),
                        memory: "128Mi".to_string(),
                    },
                    limits: ResourceQuantity {
                        cpu: "500m".to_string(),
                        memory: "512Mi".to_string(),
                    },
                },
                config: ClusterConfig {
                    server: ServerConfig {
                        port: 3000,
                        host: "0.0.0.0".to_string(),
                        max_connections: 1000,
                        timeout_seconds: 30,
                    },
                    engine: EngineConfig {
                        max_concurrent_tasks: 10,
                        reasoning_timeout_seconds: 60,
                        batch_size: 100,
                    },
                    security: SecurityConfig::default(),
                    env_vars: HashMap::new(),
                },
                storage: StorageSpec {
                    storage_type: StorageType::PersistentVolume,
                    storage_class: Some("standard".to_string()),
                    size: "10Gi".to_string(),
                    pvc: PvcSpec::default(),
                },
                monitoring: MonitoringSpec::default(),
                scaling: ScalingSpec::default(),
                network: NetworkSpec::default(),
            },
            status: None,
        };

        let reconciler = FukurowReconciler::new(Client::try_default().unwrap(), OperatorConfig::default());
        let config_data = reconciler.generate_config_data(&cluster);

        assert_eq!(config_data.get("FUROW_SERVER_PORT"), Some(&"3000".to_string()));
        assert_eq!(config_data.get("FUROW_MAX_CONCURRENT_TASKS"), Some(&"10".to_string()));
        assert_eq!(config_data.get("FUROW_METRICS_ENABLED"), Some(&"true".to_string()));
    }

    #[test]
    fn test_cluster_labels() {
        let cluster = FukurowCluster {
            metadata: k8s_openapi::apimachinery::pkg::apis::meta::v1::ObjectMeta {
                name: Some("test-cluster".to_string()),
                namespace: Some("default".to_string()),
                ..Default::default()
            },
            spec: FukurowClusterSpec {
                replicas: 3,
                image: ImageSpec {
                    registry: "ghcr.io/gftdcojp".to_string(),
                    repository: "fukurow".to_string(),
                    tag: "latest".to_string(),
                    pull_policy: PullPolicy::IfNotPresent,
                },
                ..Default::default()
            },
            status: None,
        };

        let reconciler = FukurowReconciler::new(Client::try_default().unwrap(), OperatorConfig::default());
        let labels = reconciler.cluster_labels(&cluster);

        assert_eq!(labels.get("app.kubernetes.io/name"), Some(&"fukurow".to_string()));
        assert_eq!(labels.get("app.kubernetes.io/instance"), Some(&"test-cluster".to_string()));
        assert_eq!(labels.get("app.kubernetes.io/version"), Some(&"latest".to_string()));
    }
}
