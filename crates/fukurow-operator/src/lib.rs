//! # Fukurow Kubernetes Operator
//!
//! Kubernetes operator for deploying and managing Fukurow reasoning engine clusters.
//! Provides automated scaling, monitoring, and lifecycle management.

pub mod crds;
pub mod controller;
pub mod manager;
pub mod reconciler;

pub use crds::*;
pub use controller::*;
pub use manager::*;
pub use reconciler::*;

/// Operator configuration
#[derive(Debug, Clone)]
pub struct OperatorConfig {
    pub namespace: String,
    pub image_registry: String,
    pub image_tag: String,
    pub enable_scaling: bool,
    pub enable_monitoring: bool,
    pub default_replicas: u32,
    pub max_replicas: u32,
    pub min_replicas: u32,
}

impl Default for OperatorConfig {
    fn default() -> Self {
        Self {
            namespace: "default".to_string(),
            image_registry: "ghcr.io/gftdcojp/fukurow".to_string(),
            image_tag: "latest".to_string(),
            enable_scaling: true,
            enable_monitoring: true,
            default_replicas: 3,
            max_replicas: 10,
            min_replicas: 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_operator_config_default() {
        let config = OperatorConfig::default();
        assert_eq!(config.namespace, "default");
        assert_eq!(config.default_replicas, 3);
        assert_eq!(config.max_replicas, 10);
        assert!(config.enable_scaling);
    }
}
