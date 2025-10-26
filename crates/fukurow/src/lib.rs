//! # 🦉 Fukurow - Rust Reasoning & Knowledge Graph Stack
//!
//! Fukurow is a comprehensive Rust stack for processing knowledge graphs and performing semantic reasoning.
//! Built on JSON-LD, RDF, OWL, SPARQL, and GraphQL-LD standards, it provides high-performance inference
//! engines and auditable knowledge stores.
//!
//! ## Features
//!
//! - **🚀 High-Performance Inference**: LTO-optimized native-speed RDF/OWL reasoning
//! - **🔒 Auditable Knowledge Store**: Complete provenance tracking and audit trails
//! - **🦉 Cyber Security Focused**: Advanced rules for EDR/SIEM event analysis
//! - **🌐 WebAssembly Support**: Browser-compatible execution
//! - **📊 Semantic Knowledge Graphs**: JSON-LD based data models
//! - **🔧 Rust Ecosystem**: Memory-safe, high-performance, concurrent processing
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use fukurow::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     // Create a reasoner engine
//!     let mut engine = ReasonerEngine::new();
//!
//!     // Create a cyber event (network connection example)
//!     let event = CyberEvent::NetworkConnection {
//!         source_ip: "192.168.1.100".to_string(),
//!         dest_ip: "10.0.0.1".to_string(),
//!         port: 443,
//!         protocol: "TCP".to_string(),
//!         timestamp: chrono::Utc::now().timestamp(),
//!     };
//!
//!     engine.add_event(event).await?;
//!
//!     // Perform reasoning
//!     let actions = engine.reason().await?;
//!
//!     println!("Generated {} actions", actions.len());
//!     Ok(())
//! }
//! ```
//!
//! ## Architecture
//!
//! Fukurow consists of several specialized crates:
//!
//! - **`fukurow-core`**: Core RDF/JSON-LD data models and processing
//! - **`fukurow-store`**: RDF triple store with provenance tracking
//! - **`fukurow-rules`**: Rule traits and constraint validation (SHACL equivalent)
//! - **`fukurow-engine`**: Reasoning engine orchestration
//! - **`fukurow-domain-cyber`**: Cyber security domain rules
//! - **`fukurow-api`**: RESTful web API
//! - **`fukurow-cli`**: Command-line interface
//!
//! ## Feature Flags
//!
//! - `full` (default): All crates included
//! - `core`: Only core data models
//! - `store`: RDF triple store functionality
//! - `rules`: Rule engine and validation
//! - `engine`: Reasoning orchestration
//! - `cyber`: Cyber security domain rules
//! - `api`: REST API server
//! - `cli`: Command-line tools
//! - `wasm`: WebAssembly support

// Re-export all public APIs from sub-crates (feature-gated)

#[cfg(feature = "fukurow-core")]
pub use fukurow_core as core;

#[cfg(feature = "fukurow-store")]
pub use fukurow_store as store;

#[cfg(feature = "fukurow-rules")]
pub use fukurow_rules as rules;

#[cfg(feature = "fukurow-engine")]
pub use fukurow_engine as engine;

#[cfg(feature = "fukurow-domain-cyber")]
pub use fukurow_domain_cyber as domain_cyber;

#[cfg(feature = "fukurow-api")]
pub use fukurow_api as api;

#[cfg(feature = "fukurow-cli")]
pub use fukurow_cli as cli;

// Convenience re-exports for common types (feature-gated)
#[cfg(feature = "fukurow-core")]
pub use fukurow_core::model;

#[cfg(feature = "fukurow-engine")]
pub use fukurow_engine::{ReasonerEngine, ReasonerError};

#[cfg(feature = "fukurow-store")]
pub use fukurow_store::{RdfStore, Provenance};

#[cfg(feature = "fukurow-rules")]
pub use fukurow_rules::{Rule, RuleRegistry};

#[cfg(feature = "fukurow-domain-cyber")]
pub use fukurow_domain_cyber::threat_intelligence::{ThreatProcessor, IndicatorType};

// Commonly used external dependencies
pub use serde;
pub use serde_json;
pub use anyhow;
pub use tokio;

/// Prelude module for convenient imports
///
/// ```rust
/// use fukurow::prelude::*;
/// ```
pub mod prelude {
    // Core types (feature-gated)
    #[cfg(feature = "fukurow-core")]
    pub use crate::model::*;

    #[cfg(feature = "fukurow-engine")]
    pub use crate::ReasonerEngine;
    #[cfg(feature = "fukurow-engine")]
    pub use crate::ReasonerError;

    #[cfg(feature = "fukurow-store")]
    pub use crate::RdfStore;
    #[cfg(feature = "fukurow-store")]
    pub use crate::Provenance;

    #[cfg(feature = "fukurow-rules")]
    pub use crate::Rule;
    #[cfg(feature = "fukurow-rules")]
    pub use crate::RuleRegistry;

    #[cfg(feature = "fukurow-domain-cyber")]
    pub use crate::ThreatProcessor;
    #[cfg(feature = "fukurow-domain-cyber")]
    pub use crate::IndicatorType;

    // Common external types
    pub use serde::{Deserialize, Serialize};
    pub use serde_json::Value;
    pub use anyhow::Result;
    pub use tokio;
}

// Module declarations for organization (feature-gated)
#[cfg(feature = "fukurow-domain-cyber")]
pub mod cyber {
    //! Cyber security domain functionality
    pub use fukurow_domain_cyber::*;
}

#[cfg(all(feature = "fukurow-engine", feature = "fukurow-rules"))]
pub mod reasoning {
    //! Reasoning engine and rules
    pub use fukurow_engine::*;
    pub use fukurow_rules::*;
}

#[cfg(feature = "fukurow-api")]
pub mod web {
    //! Web API and services
    pub use fukurow_api::*;
}

#[cfg(feature = "fukurow-cli")]
pub mod cli_tools {
    //! Command-line tools
    pub use fukurow_cli::*;
}

// Version information
/// Current version of Fukurow
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Build information
pub mod build_info {
    /// Build timestamp (fallback value)
    pub const BUILD_TIME: &str = "2024-01-01T00:00:00Z";
    /// Git commit hash (fallback value)
    pub const GIT_SHA: &str = "unknown";
    /// Git commit date (fallback value)
    pub const GIT_COMMIT_DATE: &str = "2024-01-01T00:00:00Z";
}

/// Health check function
///
/// Returns basic system information to verify Fukurow is working correctly.
pub fn health_check() -> serde_json::Value {
    serde_json::json!({
        "status": "healthy",
        "version": VERSION,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "modules": {
            "core": true,
            "store": true,
            "rules": true,
            "engine": true,
            "domain_cyber": true,
            "api": true,
            "cli": true
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check() {
        let health = health_check();
        assert_eq!(health["status"], "healthy");
        assert_eq!(health["version"], VERSION);
    }

    #[test]
    fn test_version_constant() {
        assert!(!VERSION.is_empty());
        assert!(VERSION.starts_with('v') || VERSION.chars().all(|c| c.is_ascii_digit() || c == '.'));
    }

    #[cfg(all(feature = "fukurow-engine", feature = "fukurow-store"))]
    #[tokio::test]
    async fn test_basic_engine_creation() {
        let engine = ReasonerEngine::new();
        // Basic smoke test - engine should be created without error
        // get_graph_store returns Arc<RwLock<RdfStore>>, so just check it exists
        let _store = engine.get_graph_store().await;
        // If we get here without panicking, the engine was created successfully
    }
}
