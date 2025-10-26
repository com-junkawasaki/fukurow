//! Google Chronicle SIEM統合

use crate::{SiemClient, SiemConfig, SiemEvent, SiemResult, SiemError};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Google Chronicle client
pub struct ChronicleClient {
    config: SiemConfig,
    client: Client,
    customer_id: String,
}

impl ChronicleClient {
    /// Create new Chronicle client
    pub fn new(config: SiemConfig, customer_id: &str) -> Self {
        Self {
            client: Client::new(),
            config,
            customer_id: customer_id.to_string(),
        }
    }

    /// Get authentication headers
    fn get_auth_headers(&self) -> Result<HashMap<String, String>, SiemError> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        if let Some(api_key) = &self.config.api_key {
            headers.insert("Authorization".to_string(), format!("Bearer {}", api_key));
        } else {
            return Err(SiemError::ConfigError("API key required for Chronicle".to_string()));
        }

        Ok(headers)
    }
}

#[async_trait]
impl SiemClient for ChronicleClient {
    async fn send_event(&self, event: SiemEvent) -> SiemResult<()> {
        let url = format!("{}/v1/projects/{}/locations/global/instances/-/udmEvents:ingest",
                         self.config.endpoint, self.customer_id);

        // Convert SiemEvent to Chronicle UDM format
        let udm_event = self.convert_to_udm(event)?;

        let mut request = self.client
            .post(&url)
            .json(&udm_event);

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

    async fn send_events(&self, events: Vec<SiemEvent>) -> SiemResult<()> {
        // Chronicle API supports batch ingestion
        let url = format!("{}/v1/projects/{}/locations/global/instances/-/udmEvents:batchCreate",
                         self.config.endpoint, self.customer_id);

        let udm_events: Result<Vec<_>, _> = events.into_iter()
            .map(|event| self.convert_to_udm(event))
            .collect();

        let udm_events = udm_events?;
        let batch_request = ChronicleBatchRequest { events: udm_events };

        let mut request = self.client
            .post(&url)
            .json(&batch_request);

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

    async fn query_events(&self, query: &str, limit: Option<usize>) -> SiemResult<Vec<SiemEvent>> {
        let url = format!("{}/v1/projects/{}/locations/global/instances/-/udmEvents:query",
                         self.config.endpoint, self.customer_id);

        let query_request = ChronicleQueryRequest {
            query: query.to_string(),
            start_time: None, // Could be parsed from query
            end_time: None,
            limit: limit.map(|l| l as i32),
        };

        let mut request = self.client
            .post(&url)
            .json(&query_request);

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

        let query_response: ChronicleQueryResponse = response.json().await?;
        let events = query_response.events
            .into_iter()
            .map(|udm| self.convert_from_udm(udm))
            .collect::<Result<Vec<_>, _>>()?;

        Ok(events)
    }

    async fn health_check(&self) -> SiemResult<bool> {
        // Chronicle doesn't have a specific health check endpoint
        // We can try a simple query to check connectivity
        let test_query = "metadata.event_type = \"NETWORK_CONNECTION\" LIMIT 1";

        match self.query_events(test_query, Some(1)).await {
            Ok(_) => Ok(true),
            Err(SiemError::ApiError { status: 403 | 401, .. }) => Ok(false), // Auth issues
            Err(SiemError::ApiError { status: 404, .. }) => Ok(false), // Endpoint issues
            Err(_) => Ok(false), // Other issues
        }
    }
}

impl ChronicleClient {
    /// Convert SiemEvent to Chronicle UDM format
    fn convert_to_udm(&self, event: SiemEvent) -> SiemResult<ChronicleUdmEvent> {
        // Create a basic UDM event structure
        // In practice, this would need more sophisticated mapping
        let udm_event = ChronicleUdmEvent {
            metadata: ChronicleMetadata {
                event_type: "GENERIC_EVENT".to_string(), // Map event.event_type to UDM types
                event_timestamp: Some(event.timestamp.timestamp_micros()),
                collected_timestamp: event.timestamp.timestamp_micros(),
                product_name: "Fukurow".to_string(),
                product_version: "1.0.0".to_string(),
                product_event_type: event.event_type.clone(),
                vendor_name: "Fukurow".to_string(),
                product_log_id: Some(event.id.clone()),
            },
            principal: None,
            target: None,
            src: None,
            network: None,
            security_result: Some(vec![ChronicleSecurityResult {
                severity: match event.severity {
                    crate::SiemSeverity::Low => "INFORMATIONAL".to_string(),
                    crate::SiemSeverity::Medium => "LOW".to_string(),
                    crate::SiemSeverity::High => "MEDIUM".to_string(),
                    crate::SiemSeverity::Critical => "HIGH".to_string(),
                },
                action: vec!["ALERT".to_string()],
                confidence: Some(0.8),
                summary: event.message.clone(),
                description: event.raw_data,
            }]),
            additional: Some(serde_json::json!({
                "fukurow_source": event.source,
                "fukurow_metadata": event.metadata
            })),
        };

        Ok(udm_event)
    }

    /// Convert Chronicle UDM to SiemEvent
    fn convert_from_udm(&self, udm: ChronicleUdmEvent) -> SiemResult<SiemEvent> {
        let severity = if let Some(security_results) = &udm.security_result {
            if let Some(first_result) = security_results.first() {
                match first_result.severity.as_str() {
                    "HIGH" | "CRITICAL" => crate::SiemSeverity::High,
                    "MEDIUM" => crate::SiemSeverity::Medium,
                    "LOW" => crate::SiemSeverity::Low,
                    _ => crate::SiemSeverity::Medium,
                }
            } else {
                crate::SiemSeverity::Medium
            }
        } else {
            crate::SiemSeverity::Medium
        };

        let message = if let Some(security_results) = &udm.security_result {
            if let Some(first_result) = security_results.first() {
                first_result.summary.clone()
            } else {
                "Chronicle event".to_string()
            }
        } else {
            "Chronicle event".to_string()
        };

        Ok(SiemEvent {
            id: udm.metadata.product_log_id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string()),
            timestamp: udm.metadata.event_timestamp
                .map(|ts| chrono::DateTime::from_timestamp_micros(ts).unwrap_or_else(|| chrono::Utc::now()))
                .unwrap_or_else(|| chrono::Utc::now()),
            event_type: udm.metadata.product_event_type,
            source: "chronicle".to_string(),
            severity,
            message,
            metadata: udm.additional.unwrap_or(serde_json::Value::Null),
            raw_data: Some(serde_json::to_string(&udm).unwrap_or_default()),
        })
    }
}

/// Chronicle UDM Event structure (simplified)
#[derive(Serialize, Deserialize)]
struct ChronicleUdmEvent {
    metadata: ChronicleMetadata,
    principal: Option<ChronicleEntity>,
    target: Option<ChronicleEntity>,
    src: Option<ChronicleEntity>,
    network: Option<ChronicleNetwork>,
    security_result: Option<Vec<ChronicleSecurityResult>>,
    additional: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct ChronicleMetadata {
    event_type: String,
    event_timestamp: Option<i64>,
    collected_timestamp: i64,
    product_name: String,
    product_version: String,
    product_event_type: String,
    vendor_name: String,
    product_log_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ChronicleEntity {
    hostname: Option<String>,
    ip: Option<Vec<String>>,
    user: Option<ChronicleUser>,
}

#[derive(Serialize, Deserialize)]
struct ChronicleUser {
    userid: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ChronicleNetwork {
    ip_protocol: Option<i32>,
    application_protocol: Option<String>,
}

#[derive(Serialize, Deserialize)]
struct ChronicleSecurityResult {
    severity: String,
    action: Vec<String>,
    confidence: Option<f64>,
    summary: String,
    description: Option<String>,
}

/// Chronicle batch request
#[derive(Serialize)]
struct ChronicleBatchRequest {
    events: Vec<ChronicleUdmEvent>,
}

/// Chronicle query request
#[derive(Serialize)]
struct ChronicleQueryRequest {
    query: String,
    start_time: Option<String>,
    end_time: Option<String>,
    limit: Option<i32>,
}

/// Chronicle query response
#[derive(Deserialize)]
struct ChronicleQueryResponse {
    events: Vec<ChronicleUdmEvent>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_chronicle_config() {
        let config = SiemConfig::new("https://chronicle.googleapis.com")
            .with_api_key("test-api-key");

        let client = ChronicleClient::new(config, "test-customer-id");
        assert_eq!(client.customer_id, "test-customer-id");
    }
}
