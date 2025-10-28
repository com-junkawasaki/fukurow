//! # SIEM Integration Module
//!
//! エンタープライズSIEMシステムとの統合機能
//! Splunk・ELK・ChronicleなどのSIEMシステムへのログ転送

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use reqwest::Client;
use tracing::{info, warn, error};

/// SIEMシステムタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SiemType {
    Splunk,
    Elk,
    Chronicle,
    Custom,
}

/// SIEM設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiemConfig {
    pub system_type: SiemType,
    pub endpoint_url: String,
    pub auth_token: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub index: Option<String>,
    pub source_type: Option<String>,
    pub custom_headers: HashMap<String, String>,
}

/// SIEM統合マネージャー
#[derive(Debug)]
pub struct SiemIntegrationManager {
    client: Client,
    configs: HashMap<String, SiemConfig>,
    enabled: bool,
}

impl SiemIntegrationManager {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
            configs: HashMap::new(),
            enabled: true,
        }
    }

    /// SIEM設定を追加
    pub fn add_config(&mut self, name: String, config: SiemConfig) {
        self.configs.insert(name.clone(), config);
        info!("Added SIEM configuration: {}", name);
    }

    /// SIEM統合を有効/無効化
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        info!("SIEM integration {}", if enabled { "enabled" } else { "disabled" });
    }

    /// すべての設定されたSIEMにイベントを送信
    pub async fn send_event(&self, event: SiemEvent) -> Result<(), SiemError> {
        if !self.enabled {
            return Ok(());
        }

        let mut errors = Vec::new();

        for (name, config) in &self.configs {
            match self.send_to_siem(name, config, &event).await {
                Ok(_) => info!("Successfully sent event to SIEM: {}", name),
                Err(e) => {
                    warn!("Failed to send event to SIEM {}: {}", name, e);
                    errors.push((name.clone(), e));
                }
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(SiemError::MultipleErrors(errors))
        }
    }

    /// 特定のSIEMにイベントを送信
    async fn send_to_siem(&self, _name: &str, config: &SiemConfig, event: &SiemEvent) -> Result<(), SiemError> {
        let payload = self.format_payload(config, event)?;
        let url = self.build_url(config)?;

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/json");

        // 認証設定
        if let Some(token) = &config.auth_token {
            match config.system_type {
                SiemType::Splunk => {
                    request = request.header("Authorization", format!("Splunk {}", token));
                }
                SiemType::Elk => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
                SiemType::Chronicle => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
                SiemType::Custom => {
                    request = request.header("Authorization", format!("Bearer {}", token));
                }
            }
        }

        // Basic認証
        if let (Some(username), Some(password)) = (&config.username, &config.password) {
            request = request.basic_auth(username, Some(password));
        }

        // カスタムヘッダー
        for (key, value) in &config.custom_headers {
            request = request.header(key, value);
        }

        let response = request
            .body(payload)
            .send()
            .await
            .map_err(|e| SiemError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(SiemError::HttpError {
                status: status.as_u16(),
                body,
            });
        }

        Ok(())
    }

    /// SIEM固有のペイロードをフォーマット
    fn format_payload(&self, config: &SiemConfig, event: &SiemEvent) -> Result<String, SiemError> {
        let payload = match config.system_type {
            SiemType::Splunk => {
                serde_json::json!({
                    "event": event,
                    "index": config.index.as_deref().unwrap_or("main"),
                    "sourcetype": config.source_type.as_deref().unwrap_or("fukurow:security"),
                    "time": event.timestamp,
                    "host": event.host,
                    "source": "fukurow-reasoner"
                })
            }
            SiemType::Elk => {
                serde_json::json!({
                    "@timestamp": event.timestamp,
                    "message": event.message,
                    "level": event.level,
                    "event": event,
                    "service": {
                        "name": "fukurow-reasoner",
                        "version": env!("CARGO_PKG_VERSION")
                    },
                    "host": {
                        "name": event.host
                    }
                })
            }
            SiemType::Chronicle => {
                serde_json::json!({
                    "event": event,
                    "metadata": {
                        "event_timestamp": event.timestamp,
                        "event_type": "SECURITY_EVENT",
                        "vendor_name": "fukurow",
                        "product_name": "reasoner"
                    }
                })
            }
            SiemType::Custom => {
                serde_json::json!(event)
            }
        };

        serde_json::to_string(&payload)
            .map_err(|e| SiemError::SerializationError(e.to_string()))
    }

    /// SIEM固有のURLを構築
    fn build_url(&self, config: &SiemConfig) -> Result<String, SiemError> {
        let base_url = config.endpoint_url.trim_end_matches('/');

        let path = match config.system_type {
            SiemType::Splunk => "/services/collector/event",
            SiemType::Elk => "/_bulk",
            SiemType::Chronicle => "/v1/ingest",
            SiemType::Custom => "",
        };

        Ok(format!("{}{}", base_url, path))
    }

    /// 設定されたSIEMの一覧を取得
    pub fn list_configs(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }

    /// 特定のSIEM設定を取得
    pub fn get_config(&self, name: &str) -> Option<&SiemConfig> {
        self.configs.get(name)
    }
}

/// SIEMイベント構造体
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SiemEvent {
    pub timestamp: String,
    pub level: String,
    pub message: String,
    pub host: String,
    pub source: String,
    pub event_type: String,
    pub severity: String,
    pub details: serde_json::Value,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl SiemEvent {
    /// セキュリティアクションからSIEMイベントを作成
    pub fn from_security_action(action: &fukurow_core::model::SecurityAction, host: String) -> Self {
        match action {
            fukurow_core::model::SecurityAction::Alert { severity, message, details } => {
                Self {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: severity.clone(),
                    message: message.clone(),
                    host,
                    source: "fukurow-reasoner".to_string(),
                    event_type: "security_alert".to_string(),
                    severity: severity.clone(),
                    details: details.clone(),
                    metadata: HashMap::new(),
                }
            }
            fukurow_core::model::SecurityAction::IsolateHost { host_ip, reason } => {
                Self {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "critical".to_string(),
                    message: format!("Host isolation required: {} - {}", host_ip, reason),
                    host: host.clone(),
                    source: "fukurow-reasoner".to_string(),
                    event_type: "host_isolation".to_string(),
                    severity: "critical".to_string(),
                    details: serde_json::json!({
                        "target_host": host_ip,
                        "reason": reason,
                        "action": "isolate"
                    }),
                    metadata: HashMap::new(),
                }
            }
            fukurow_core::model::SecurityAction::BlockConnection { source_ip, dest_ip, reason } => {
                Self {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "high".to_string(),
                    message: format!("Connection blocked: {} -> {} - {}", source_ip, dest_ip, reason),
                    host: host.clone(),
                    source: "fukurow-reasoner".to_string(),
                    event_type: "connection_block".to_string(),
                    severity: "high".to_string(),
                    details: serde_json::json!({
                        "source_ip": source_ip,
                        "dest_ip": dest_ip,
                        "reason": reason,
                        "action": "block"
                    }),
                    metadata: HashMap::new(),
                }
            }
            fukurow_core::model::SecurityAction::TerminateProcess { process_id, reason } => {
                Self {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "high".to_string(),
                    message: format!("Process termination required: {} - {}", process_id, reason),
                    host: host.clone(),
                    source: "fukurow-reasoner".to_string(),
                    event_type: "process_termination".to_string(),
                    severity: "high".to_string(),
                    details: serde_json::json!({
                        "process_id": process_id,
                        "reason": reason,
                        "action": "terminate"
                    }),
                    metadata: HashMap::new(),
                }
            }
            fukurow_core::model::SecurityAction::RevokePrivileges { user, privilege, reason } => {
                Self {
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    level: "medium".to_string(),
                    message: format!("Privilege revocation required: {} - {} - {}", user, privilege, reason),
                    host: host.clone(),
                    source: "fukurow-reasoner".to_string(),
                    event_type: "privilege_revocation".to_string(),
                    severity: "medium".to_string(),
                    details: serde_json::json!({
                        "user": user,
                        "privilege": privilege,
                        "reason": reason,
                        "action": "revoke"
                    }),
                    metadata: HashMap::new(),
                }
            }
        }
    }

    /// 異常検知結果からSIEMイベントを作成
    pub fn from_anomaly_result(result: &::fukurow_domain_cyber::anomaly_detection::AnomalyResult, host: String) -> Self {
        Self {
            timestamp: chrono::Utc::now().to_rfc3339(),
            level: if result.score > 2.0 { "high" } else { "medium" }.to_string(),
            message: format!("Anomaly detected in {}: score={:.2}", result.label, result.score),
            host,
            source: "fukurow-anomaly-detector".to_string(),
            event_type: "anomaly_detection".to_string(),
            severity: if result.score > 2.0 { "high" } else { "medium" }.to_string(),
            details: serde_json::json!({
                "anomaly_score": result.score,
                "threshold": result.threshold,
                "is_anomaly": result.is_anomaly,
                "method": result.method,
                "value": result.value,
                "label": result.label
            }),
            metadata: HashMap::new(),
        }
    }
}

/// SIEMエラー
#[derive(Debug, thiserror::Error)]
pub enum SiemError {
    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("HTTP error: status={status}, body={body}")]
    HttpError { status: u16, body: String },

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Multiple errors: {0:?}")]
    MultipleErrors(Vec<(String, SiemError)>),
}

/// SIEM統合ユーティリティ関数
pub struct SiemUtils;

impl SiemUtils {
    /// Splunk設定を作成
    pub fn create_splunk_config(endpoint_url: String, auth_token: String) -> SiemConfig {
        SiemConfig {
            system_type: SiemType::Splunk,
            endpoint_url,
            auth_token: Some(auth_token),
            username: None,
            password: None,
            index: Some("security".to_string()),
            source_type: Some("fukurow:security".to_string()),
            custom_headers: HashMap::new(),
        }
    }

    /// ELK設定を作成
    pub fn create_elk_config(endpoint_url: String, auth_token: Option<String>) -> SiemConfig {
        SiemConfig {
            system_type: SiemType::Elk,
            endpoint_url,
            auth_token,
            username: None,
            password: None,
            index: None,
            source_type: None,
            custom_headers: HashMap::new(),
        }
    }

    /// Chronicle設定を作成
    pub fn create_chronicle_config(endpoint_url: String, auth_token: String) -> SiemConfig {
        SiemConfig {
            system_type: SiemType::Chronicle,
            endpoint_url,
            auth_token: Some(auth_token),
            username: None,
            password: None,
            index: None,
            source_type: None,
            custom_headers: HashMap::new(),
        }
    }

    /// バッチイベント送信（パフォーマンス最適化）
    pub async fn send_batch_events(
        manager: &SiemIntegrationManager,
        events: Vec<SiemEvent>
    ) -> Result<(), SiemError> {
        for event in events {
            if let Err(e) = manager.send_event(event).await {
                error!("Failed to send batch event: {}", e);
                // 個別のイベント失敗を無視して継続
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fukurow_core::model::SecurityAction;

    #[test]
    fn test_siem_event_from_security_action() {
        let action = SecurityAction::Alert {
            severity: "high".to_string(),
            message: "Test alert".to_string(),
            details: serde_json::json!({"test": "data"}),
        };

        let event = SiemEvent::from_security_action(&action, "testhost".to_string());

        assert_eq!(event.level, "high");
        assert_eq!(event.message, "Test alert");
        assert_eq!(event.host, "testhost");
        assert_eq!(event.source, "fukurow-reasoner");
        assert_eq!(event.event_type, "security_alert");
    }

    #[test]
    fn test_splunk_config_creation() {
        let config = SiemUtils::create_splunk_config(
            "https://splunk.example.com:8088".to_string(),
            "token123".to_string()
        );

        assert!(matches!(config.system_type, SiemType::Splunk));
        assert_eq!(config.endpoint_url, "https://splunk.example.com:8088");
        assert_eq!(config.auth_token, Some("token123".to_string()));
        assert_eq!(config.index, Some("security".to_string()));
    }

    #[test]
    fn test_elk_config_creation() {
        let config = SiemUtils::create_elk_config(
            "https://elk.example.com:9200".to_string(),
            Some("token456".to_string())
        );

        assert!(matches!(config.system_type, SiemType::Elk));
        assert_eq!(config.endpoint_url, "https://elk.example.com:9200");
        assert_eq!(config.auth_token, Some("token456".to_string()));
    }

    #[test]
    fn test_chronicle_config_creation() {
        let config = SiemUtils::create_chronicle_config(
            "https://chronicle.example.com".to_string(),
            "token789".to_string()
        );

        assert!(matches!(config.system_type, SiemType::Chronicle));
        assert_eq!(config.endpoint_url, "https://chronicle.example.com");
        assert_eq!(config.auth_token, Some("token789".to_string()));
    }

    #[test]
    fn test_siem_manager_operations() {
        let mut manager = SiemIntegrationManager::new();

        let config = SiemUtils::create_splunk_config(
            "https://splunk.example.com".to_string(),
            "token".to_string()
        );

        manager.add_config("test_splunk".to_string(), config);

        let configs = manager.list_configs();
        assert_eq!(configs.len(), 1);
        assert_eq!(configs[0], "test_splunk");

        let retrieved = manager.get_config("test_splunk");
        assert!(retrieved.is_some());
        assert!(matches!(retrieved.unwrap().system_type, SiemType::Splunk));
    }

    #[tokio::test]
    async fn test_disabled_siem_integration() {
        let mut manager = SiemIntegrationManager::new();
        manager.set_enabled(false);

        let action = SecurityAction::Alert {
            severity: "low".to_string(),
            message: "Test".to_string(),
            details: serde_json::json!({}),
        };

        let event = SiemEvent::from_security_action(&action, "test".to_string());

        // 無効化されているのでエラーにならない
        let result = manager.send_event(event).await;
        assert!(result.is_ok());
    }
}
