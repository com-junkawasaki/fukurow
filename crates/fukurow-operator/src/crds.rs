//! # Custom Resource Definitions
//!
//! Kubernetes CRDs for Fukurow operator

use kube::CustomResource;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// FukurowCluster CRD - Main cluster resource
#[derive(CustomResource, Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[kube(
    group = "fukurow.io",
    version = "v1",
    kind = "FukurowCluster",
    plural = "fukurowclusters",
    derive = "Default",
    namespaced
)]
#[kube(status = "FukurowClusterStatus")]
#[serde(rename_all = "camelCase")]
pub struct FukurowClusterSpec {
    /// Number of replicas
    pub replicas: u32,

    /// Image configuration
    pub image: ImageSpec,

    /// Resource requirements
    pub resources: ResourceRequirements,

    /// Configuration for the cluster
    pub config: ClusterConfig,

    /// Storage configuration
    pub storage: StorageSpec,

    /// Monitoring configuration
    #[serde(default)]
    pub monitoring: MonitoringSpec,

    /// Scaling configuration
    #[serde(default)]
    pub scaling: ScalingSpec,

    /// Network configuration
    #[serde(default)]
    pub network: NetworkSpec,
}

/// Image specification
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ImageSpec {
    /// Container registry
    pub registry: String,

    /// Repository name
    pub repository: String,

    /// Image tag
    pub tag: String,

    /// Image pull policy
    #[serde(default)]
    pub pull_policy: PullPolicy,
}

/// Pull policy for container images
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
pub enum PullPolicy {
    #[default]
    #[serde(rename = "IfNotPresent")]
    IfNotPresent,

    #[serde(rename = "Always")]
    Always,

    #[serde(rename = "Never")]
    Never,
}

/// Resource requirements
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirements {
    /// CPU requests
    pub requests: ResourceQuantity,

    /// CPU limits
    pub limits: ResourceQuantity,
}

/// Resource quantity specification
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ResourceQuantity {
    /// CPU quantity (e.g., "100m", "0.1")
    pub cpu: String,

    /// Memory quantity (e.g., "128Mi", "1Gi")
    pub memory: String,
}

/// Cluster configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClusterConfig {
    /// Server configuration
    pub server: ServerConfig,

    /// Engine configuration
    pub engine: EngineConfig,

    /// Security configuration
    #[serde(default)]
    pub security: SecurityConfig,

    /// Additional environment variables
    #[serde(default)]
    pub env_vars: HashMap<String, String>,
}

/// Server configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ServerConfig {
    /// HTTP port
    pub port: u16,

    /// Host binding
    #[serde(default = "default_host")]
    pub host: String,

    /// Maximum connections
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,

    /// Request timeout in seconds
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_max_connections() -> u32 {
    1000
}

fn default_timeout() -> u32 {
    30
}

/// Engine configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EngineConfig {
    /// Maximum concurrent reasoning tasks
    #[serde(default = "default_max_tasks")]
    pub max_concurrent_tasks: usize,

    /// Reasoning timeout in seconds
    #[serde(default = "default_reasoning_timeout")]
    pub reasoning_timeout_seconds: u32,

    /// Rule evaluation batch size
    #[serde(default = "default_batch_size")]
    pub batch_size: usize,
}

fn default_max_tasks() -> usize {
    10
}

fn default_reasoning_timeout() -> u32 {
    60
}

fn default_batch_size() -> usize {
    100
}

/// Security configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct SecurityConfig {
    /// Enable TLS
    #[serde(default)]
    pub tls_enabled: bool,

    /// TLS certificate secret name
    pub tls_secret_name: Option<String>,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// SIEM integration
    #[serde(default)]
    pub siem: SiemConfig,
}

/// Authentication configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct AuthConfig {
    /// Enable authentication
    #[serde(default)]
    pub enabled: bool,

    /// JWT secret name
    pub jwt_secret_name: Option<String>,

    /// OAuth2 configuration
    pub oauth2: Option<OAuth2Config>,
}

/// OAuth2 configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2Config {
    /// Provider (google, github, etc.)
    pub provider: String,

    /// Client ID
    pub client_id: String,

    /// Client secret name
    pub client_secret_name: String,

    /// Redirect URL
    pub redirect_url: String,
}

/// SIEM configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct SiemConfig {
    /// Enable SIEM integration
    #[serde(default)]
    pub enabled: bool,

    /// SIEM endpoints
    #[serde(default)]
    pub endpoints: Vec<SiemEndpoint>,
}

/// SIEM endpoint configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SiemEndpoint {
    /// Endpoint name
    pub name: String,

    /// SIEM type (splunk, elk, chronicle)
    pub siem_type: String,

    /// Endpoint URL
    pub url: String,

    /// Authentication token secret name
    pub token_secret_name: Option<String>,
}

/// Storage configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct StorageSpec {
    /// Storage type
    pub storage_type: StorageType,

    /// Storage class name
    pub storage_class: Option<String>,

    /// Storage size
    pub size: String,

    /// PVC configuration
    #[serde(default)]
    pub pvc: PvcSpec,
}

/// Storage type
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
pub enum StorageType {
    #[serde(rename = "persistentVolume")]
    PersistentVolume,

    #[serde(rename = "emptyDir")]
    EmptyDir,

    #[serde(rename = "hostPath")]
    HostPath,

    #[serde(rename = "configMap")]
    ConfigMap,
}

/// PVC specification
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct PvcSpec {
    /// Access modes
    #[serde(default = "default_access_modes")]
    pub access_modes: Vec<String>,

    /// Volume mode
    #[serde(default = "default_volume_mode")]
    pub volume_mode: String,
}

fn default_access_modes() -> Vec<String> {
    vec!["ReadWriteOnce".to_string()]
}

fn default_volume_mode() -> String {
    "Filesystem".to_string()
}

/// Monitoring configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct MonitoringSpec {
    /// Enable Prometheus metrics
    #[serde(default)]
    pub prometheus_enabled: bool,

    /// Metrics port
    #[serde(default = "default_metrics_port")]
    pub metrics_port: u16,

    /// Enable health checks
    #[serde(default = "default_true")]
    pub health_checks_enabled: bool,

    /// Health check port
    #[serde(default = "default_health_port")]
    pub health_port: u16,

    /// Enable tracing
    #[serde(default)]
    pub tracing_enabled: bool,

    /// Tracing endpoint
    pub tracing_endpoint: Option<String>,
}

fn default_metrics_port() -> u16 {
    9090
}

fn default_health_port() -> u16 {
    8080
}

fn default_true() -> bool {
    true
}

/// Scaling configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct ScalingSpec {
    /// Enable horizontal pod autoscaling
    #[serde(default)]
    pub hpa_enabled: bool,

    /// Minimum replicas
    #[serde(default = "default_min_replicas")]
    pub min_replicas: u32,

    /// Maximum replicas
    #[serde(default = "default_max_replicas")]
    pub max_replicas: u32,

    /// Target CPU utilization percentage
    #[serde(default = "default_target_cpu")]
    pub target_cpu_utilization: u32,

    /// Target memory utilization percentage
    #[serde(default = "default_target_memory")]
    pub target_memory_utilization: u32,

    /// Scale up threshold
    #[serde(default = "default_scale_up_threshold")]
    pub scale_up_threshold: f64,

    /// Scale down threshold
    #[serde(default = "default_scale_down_threshold")]
    pub scale_down_threshold: f64,
}

fn default_min_replicas() -> u32 {
    1
}

fn default_max_replicas() -> u32 {
    10
}

fn default_target_cpu() -> u32 {
    70
}

fn default_target_memory() -> u32 {
    80
}

fn default_scale_up_threshold() -> f64 {
    0.8
}

fn default_scale_down_threshold() -> f64 {
    0.3
}

/// Network configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSpec {
    /// Service type
    #[serde(default = "default_service_type")]
    pub service_type: String,

    /// Service annotations
    #[serde(default)]
    pub service_annotations: HashMap<String, String>,

    /// Ingress configuration
    pub ingress: Option<IngressSpec>,

    /// Network policies
    #[serde(default)]
    pub network_policies: Vec<NetworkPolicySpec>,
}

fn default_service_type() -> String {
    "ClusterIP".to_string()
}

/// Ingress specification
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IngressSpec {
    /// Enable ingress
    #[serde(default)]
    pub enabled: bool,

    /// Ingress class name
    pub class_name: Option<String>,

    /// Hostnames
    #[serde(default)]
    pub hosts: Vec<String>,

    /// TLS configuration
    #[serde(default)]
    pub tls: Vec<IngressTls>,

    /// Annotations
    #[serde(default)]
    pub annotations: HashMap<String, String>,
}

/// Ingress TLS configuration
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct IngressTls {
    /// Secret name
    pub secret_name: String,

    /// Hostnames
    pub hosts: Vec<String>,
}

/// Network policy specification
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct NetworkPolicySpec {
    /// Policy name
    pub name: String,

    /// Policy spec
    pub spec: serde_json::Value, // Kubernetes NetworkPolicy spec
}

/// Cluster status
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
#[serde(rename_all = "camelCase")]
pub struct FukurowClusterStatus {
    /// Current phase
    pub phase: ClusterPhase,

    /// Number of ready replicas
    pub ready_replicas: u32,

    /// Total number of replicas
    pub replicas: u32,

    /// Available replicas
    pub available_replicas: u32,

    /// Unavailable replicas
    pub unavailable_replicas: u32,

    /// Conditions
    #[serde(default)]
    pub conditions: Vec<ClusterCondition>,

    /// Last update timestamp
    pub last_update: Option<String>,

    /// Version information
    pub version: Option<String>,
}

/// Cluster phase
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema, Default)]
pub enum ClusterPhase {
    #[default]
    #[serde(rename = "Pending")]
    Pending,

    #[serde(rename = "Creating")]
    Creating,

    #[serde(rename = "Running")]
    Running,

    #[serde(rename = "Updating")]
    Updating,

    #[serde(rename = "Failed")]
    Failed,

    #[serde(rename = "Deleting")]
    Deleting,
}

/// Cluster condition
#[derive(Deserialize, Serialize, Clone, Debug, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ClusterCondition {
    /// Condition type
    pub type_: String,

    /// Status
    pub status: String,

    /// Last transition time
    pub last_transition_time: String,

    /// Reason
    pub reason: String,

    /// Message
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fukurow_cluster_spec_serialization() {
        let spec = FukurowClusterSpec {
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
        };

        let json = serde_json::to_string(&spec).unwrap();
        let deserialized: FukurowClusterSpec = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.replicas, 3);
        assert_eq!(deserialized.image.repository, "fukurow");
        assert_eq!(deserialized.config.server.port, 3000);
    }

    #[test]
    fn test_cluster_status_default() {
        let status = FukurowClusterStatus::default();
        assert!(matches!(status.phase, ClusterPhase::Pending));
        assert_eq!(status.ready_replicas, 0);
        assert_eq!(status.replicas, 0);
    }
}
