//! Splunk SIEM統合

use crate::{SiemClient, SiemConfig, SiemEvent, SiemResult, SiemError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Splunk client supporting both REST API and HEC
pub struct SplunkClient {
    config: SiemConfig,
    client: Client,
    use_hec: bool,
    hec_token: Option<String>,
}

impl SplunkClient {
    /// Create new Splunk client using REST API
    pub fn new_rest(config: SiemConfig) -> Self {
        Self {
            client: Client::new(),
            config,
            use_hec: false,
            hec_token: None,
        }
    }

    /// Create new Splunk client using HEC (HTTP Event Collector)
    pub fn new_hec(config: SiemConfig, hec_token: &str) -> Self {
        Self {
            client: Client::new(),
            config,
            use_hec: true,
            hec_token: Some(hec_token.to_string()),
        }
    }

    /// Get authentication headers for REST API
    fn get_auth_headers(&self) -> Result<HashMap<String, String>, SiemError> {
        let mut headers = HashMap::new();

        if let (Some(username), Some(password)) = (&self.config.username, &self.config.password) {
            use base64::{Engine as _, engine::general_purpose};
            let credentials = format!("{}:{}", username, password);
            let encoded = general_purpose::STANDARD.encode(credentials);
            headers.insert("Authorization".to_string(), format!("Basic {}", encoded));
        } else {
            return Err(SiemError::ConfigError("Username and password required for Splunk REST API".to_string()));
        }

        headers.insert("Content-Type".to_string(), "application/json".to_string());
        Ok(headers)
    }
}

#[async_trait]
impl SiemClient for SplunkClient {
    async fn send_event(&self, event: SiemEvent) -> SiemResult<()> {
        if self.use_hec {
            self.send_via_hec(event).await
        } else {
            self.send_via_rest(event).await
        }
    }

    async fn send_events(&self, events: Vec<SiemEvent>) -> SiemResult<()> {
        for event in events {
            self.send_event(event).await?;
        }
        Ok(())
    }

    async fn query_events(&self, query: &str, limit: Option<usize>) -> SiemResult<Vec<SiemEvent>> {
        if self.use_hec {
            return Err(SiemError::ApiError {
                status: 400,
                message: "HEC does not support querying".to_string(),
            });
        }

        self.query_via_rest(query, limit).await
    }

    async fn health_check(&self) -> SiemResult<bool> {
        let url = if self.use_hec {
            format!("{}/services/collector/health", self.config.endpoint)
        } else {
            format!("{}/services/server/info", self.config.endpoint)
        };

        let request = if self.use_hec {
            self.client
                .get(&url)
                .header("Authorization", format!("Splunk {}", self.hec_token.as_ref().unwrap()))
        } else {
            let mut request = self.client.get(&url);
            if let Ok(headers) = self.get_auth_headers() {
                for (key, value) in headers {
                    request = request.header(&key, &value);
                }
            }
            request
        };

        let response = request
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await?;

        Ok(response.status().is_success())
    }
}

impl SplunkClient {
    /// Send event via HEC (HTTP Event Collector)
    async fn send_via_hec(&self, event: SiemEvent) -> SiemResult<()> {
        let url = format!("{}/services/collector/event", self.config.endpoint);

        let hec_event = SplunkHecEvent {
            event: serde_json::to_string(&event).map_err(|e| SiemError::ParseError(e.to_string()))?,
            sourcetype: Some("_json".to_string()),
            source: Some(event.source.clone()),
            index: Some("main".to_string()),
            host: Some("fukurow".to_string()),
        };

        let response = self.client
            .post(&url)
            .header("Authorization", format!("Splunk {}", self.hec_token.as_ref().unwrap()))
            .json(&hec_event)
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

    /// Send event via REST API
    async fn send_via_rest(&self, event: SiemEvent) -> SiemResult<()> {
        let url = format!("{}/services/receivers/simple", self.config.endpoint);

        let mut request = self.client
            .post(&url)
            .query(&[("sourcetype", "_json"), ("source", &event.source), ("index", "main")])
            .body(serde_json::to_string(&event).map_err(|e| SiemError::ParseError(e.to_string()))?);

        if let Ok(headers) = self.get_auth_headers() {
            for (key, value) in headers {
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

    /// Query events via REST API
    async fn query_via_rest(&self, query: &str, limit: Option<usize>) -> SiemResult<Vec<SiemEvent>> {
        let url = format!("{}/services/search/jobs", self.config.endpoint);

        // First, create a search job
        let search_request = SplunkSearchRequest {
            search: format!("search {}", query),
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
            max_count: limit.unwrap_or(100),
        };

        let mut request = self.client
            .post(&url)
            .json(&search_request);

        if let Ok(headers) = self.get_auth_headers() {
            for (key, value) in headers {
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

        let search_response: SplunkSearchResponse = response.json().await?;
        let job_sid = search_response.sid;

        // Wait for job completion and get results
        // This is a simplified implementation
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;

        let results_url = format!("{}/services/search/jobs/{}/results", self.config.endpoint, job_sid);
        let mut results_request = self.client.get(&results_url);

        if let Ok(headers) = self.get_auth_headers() {
            for (key, value) in headers {
                results_request = results_request.header(&key, &value);
            }
        }

        let results_response = results_request
            .timeout(std::time::Duration::from_secs(self.config.timeout_seconds))
            .send()
            .await?;

        if !results_response.status().is_success() {
            let status = results_response.status().as_u16();
            let text = results_response.text().await.unwrap_or_default();
            return Err(SiemError::ApiError { status, message: text });
        }

        // Parse results (simplified)
        let results: SplunkSearchResults = results_response.json().await?;
        let events = results
            .results
            .into_iter()
            .map(|result| SiemEvent {
                id: result.get("_cd").unwrap_or(&"unknown".to_string()).clone(),
                timestamp: chrono::Utc::now(), // Parse from _time if available
                event_type: "search_result".to_string(),
                source: "splunk".to_string(),
                severity: crate::SiemSeverity::Medium,
                message: serde_json::to_string(&result).unwrap_or_default(),
                metadata: serde_json::Value::Object(serde_json::Map::new()),
                raw_data: Some(serde_json::to_string(&result).unwrap_or_default()),
            })
            .collect();

        Ok(events)
    }
}

/// Splunk HEC event format
#[derive(Serialize)]
struct SplunkHecEvent {
    event: String,
    sourcetype: Option<String>,
    source: Option<String>,
    index: Option<String>,
    host: Option<String>,
}

/// Splunk search request
#[derive(Serialize)]
struct SplunkSearchRequest {
    search: String,
    earliest_time: String,
    latest_time: String,
    max_count: usize,
}

/// Splunk search response
#[derive(Deserialize)]
struct SplunkSearchResponse {
    sid: String,
}

/// Splunk search results
#[derive(Deserialize)]
struct SplunkSearchResults {
    results: Vec<HashMap<String, String>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_splunk_config() {
        let config = SiemConfig::new("https://splunk.example.com:8089")
            .with_credentials("admin", "password");

        let client = SplunkClient::new_rest(config);
        assert!(!client.use_hec);
        assert!(client.hec_token.is_none());
    }

    #[tokio::test]
    async fn test_splunk_hec_config() {
        let config = SiemConfig::new("https://splunk.example.com:8088");
        let client = SplunkClient::new_hec(config, "test-token");

        assert!(client.use_hec);
        assert_eq!(client.hec_token, Some("test-token".to_string()));
    }
}
