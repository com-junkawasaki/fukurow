//! ELK Stack (Elasticsearch) SIEM統合

use crate::{SiemClient, SiemConfig, SiemEvent, SiemResult, SiemError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// ELK Stack (Elasticsearch) client
pub struct ElkClient {
    config: SiemConfig,
    client: Client,
    index_name: String,
}

impl ElkClient {
    /// Create new ELK client
    pub fn new(config: SiemConfig, index_name: &str) -> Self {
        Self {
            client: Client::new(),
            config,
            index_name: index_name.to_string(),
        }
    }

    /// Get authentication headers
    fn get_auth_headers(&self) -> HashMap<String, String> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            use base64::{Engine as _, engine::general_purpose};
            let credentials = format!("{}:{}", username, password);
            let encoded = general_purpose::STANDARD.encode(credentials);
            headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
        } else if let Some(api_key) = &self.config.api_key {
            headers.insert("Authorization".to_string(), format!("ApiKey {}", api_key));
        }

        headers
    }
}

#[async_trait]
impl SiemClient for ElkClient {
    async fn send_event(&self, event: SiemEvent) -> SiemResult<()> {
        let url = format!("{}/{}/_doc", self.config.endpoint, self.index_name);

        // Convert SiemEvent to Elasticsearch document
        let doc = ElasticsearchDocument {
            id: event.id.clone(),
            timestamp: event.timestamp.to_rfc3339(),
            event_type: event.event_type.clone(),
            source: event.source.clone(),
            severity: format!("{:?}", event.severity),
            message: event.message.clone(),
            metadata: event.metadata.clone(),
            raw_data: event.raw_data.clone(),
        };

        let mut request = self.client.post(&url).json(&doc);

        let headers = self.get_auth_headers();
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(SiemError::ApiError { status, message: text });
        }

        Ok(())
    }

    async fn send_events(&self, events: Vec<SiemEvent>) -> SiemResult<()> {
        let url = format!("{}/{}/_bulk", self.config.endpoint, self.index_name);

        // Prepare bulk request body
        let mut bulk_body = String::new();

        for event in events {
            // Action line
            let action = serde_json::json!({
                "index": {
                    "_index": self.index_name,
                    "_id": event.id
                }
            });
            bulk_body.push_str(&serde_json::to_string(&action)?);
            bulk_body.push('\n');

            // Document line
            let doc = serde_json::json!({
                "@timestamp": event.timestamp.to_rfc3339(),
                "event_type": event.event_type,
                "source": event.source,
                "severity": format!("{:?}", event.severity),
                "message": event.message,
                "metadata": event.metadata,
                "raw_data": event.raw_data
            });
            bulk_body.push_str(&serde_json::to_string(&doc)?);
            bulk_body.push('\n');
        }

        let mut request = self.client
            .post(&url)
            .header("Content-Type", "application/x-ndjson")
            .body(bulk_body);

        let headers = self.get_auth_headers();
        for (key, value) in headers {
            if key.to_lowercase() != "content-type" { // Don't override content-type
                request = request.header(&key, &value);
            }
        }

        let response = request
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(SiemError::ApiError { status, message: text });
        }

        Ok(())
    }

    async fn query_events(&self, query: &str, limit: Option<usize>) -> SiemResult<Vec<SiemEvent>> {
        let url = format!("{}/{}/_search", self.config.endpoint, self.index_name);

        let search_request = ElasticsearchSearchRequest {
            query: serde_json::from_str(query).unwrap_or_else(|_| {
                // Fallback to simple query_string query
                serde_json::json!({
                    "query_string": {
                        "query": query
                    }
                })
            }),
            size: limit.unwrap_or(100),
            sort: vec![serde_json::json!({
                "@timestamp": {
                    "order": "desc"
                }
            })],
        };

        let mut request = self.client.post(&url).json(&search_request);

        let headers = self.get_auth_headers();
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let text = response.text().await.unwrap_or_default();
            return Err(SiemError::ApiError { status, message: text });
        }

        let search_response: ElasticsearchSearchResponse = response.json().await?;
        let events = search_response.hits.hits
            .into_iter()
            .map(|hit| {
                let source = hit._source;
                SiemEvent {
                    id: hit._id,
                    timestamp: chrono::DateTime::parse_from_rfc3339(&source.timestamp)
                        .map(|dt| dt.with_timezone(&chrono::Utc))
                        .unwrap_or_else(|_| chrono::Utc::now()),
                    event_type: source.event_type,
                    source: source.source,
                    severity: match source.severity.as_str() {
                        "Critical" => crate::SiemSeverity::Critical,
                        "High" => crate::SiemSeverity::High,
                        "Medium" => crate::SiemSeverity::Medium,
                        _ => crate::SiemSeverity::Low,
                    },
                    message: source.message,
                    metadata: source.metadata,
                    raw_data: source.raw_data,
                }
            })
            .collect();

        Ok(events)
    }

    async fn health_check(&self) -> SiemResult<bool> {
        let url = format!("{}/_cluster/health", self.config.endpoint);

        let mut request = self.client.get(&url);

        let headers = self.get_auth_headers();
        for (key, value) in headers {
            request = request.header(&key, &value);
        }

        let response = request
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await?;

        if !response.status().is_success() {
            return Ok(false);
        }

        let health: ElasticsearchHealthResponse = response.json().await?;
        Ok(health.status == "green" || health.status == "yellow")
    }
}

/// Elasticsearch document structure
#[derive(Serialize, Deserialize)]
struct ElasticsearchDocument {
    id: String,
    timestamp: String,
    event_type: String,
    source: String,
    severity: String,
    message: String,
    metadata: serde_json::Value,
    raw_data: Option<String>,
}

/// Elasticsearch search request
#[derive(Serialize)]
struct ElasticsearchSearchRequest {
    query: serde_json::Value,
    size: usize,
    sort: Vec<serde_json::Value>,
}

/// Elasticsearch search response
#[derive(Deserialize)]
struct ElasticsearchSearchResponse {
    hits: ElasticsearchHits,
}

#[derive(Deserialize)]
struct ElasticsearchHits {
    hits: Vec<ElasticsearchHit>,
}

#[derive(Deserialize)]
struct ElasticsearchHit {
    _id: String,
    _source: ElasticsearchDocument,
}

/// Elasticsearch health response
#[derive(Deserialize)]
struct ElasticsearchHealthResponse {
    status: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_elk_config() {
        let config = SiemConfig::new("https://elasticsearch.example.com:9200")
            .with_api_key("test-api-key");

        let client = ElkClient::new(config, "fukurow-events");
        assert_eq!(client.index_name, "fukurow-events");
    }

    #[tokio::test]
    async fn test_elk_config_with_credentials() {
        let config = SiemConfig::new("https://elasticsearch.example.com:9200")
            .with_credentials("elastic", "password");

        let client = ElkClient::new(config, "security-events");
        assert_eq!(client.index_name, "security-events");
    }
}
