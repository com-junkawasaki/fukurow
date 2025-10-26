//! Threat intelligence integration

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Threat intelligence source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatSource {
    pub name: String,
    pub url: String,
    pub last_updated: i64,
    pub confidence: f64,
}

/// Threat indicator
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThreatIndicator {
    pub id: String,
    pub indicator_type: IndicatorType,
    pub value: String,
    pub threat_type: String,
    pub severity: String,
    pub sources: Vec<String>,
    pub first_seen: i64,
    pub last_seen: i64,
    pub tags: Vec<String>,
}

/// Type of threat indicator
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndicatorType {
    IpAddress,
    Domain,
    Url,
    FileHash,
    Email,
    UserAgent,
}

/// Threat intelligence feed
pub struct ThreatFeed {
    indicators: HashMap<String, ThreatIndicator>,
    sources: Vec<ThreatSource>,
}

impl ThreatFeed {
    pub fn new() -> Self {
        Self {
            indicators: HashMap::new(),
            sources: Vec::new(),
        }
    }

    /// Add threat indicator
    pub fn add_indicator(&mut self, indicator: ThreatIndicator) {
        self.indicators.insert(indicator.id.clone(), indicator);
    }

    /// Check if value is a known threat
    pub fn is_threat(&self, value: &str, indicator_type: IndicatorType) -> Option<&ThreatIndicator> {
        for indicator in self.indicators.values() {
            if indicator.indicator_type == indicator_type && indicator.value == value {
                return Some(indicator);
            }
        }
        None
    }

    /// Get all indicators of a specific type
    pub fn get_indicators_by_type(&self, indicator_type: IndicatorType) -> Vec<&ThreatIndicator> {
        self.indicators.values()
            .filter(|indicator| indicator.indicator_type == indicator_type)
            .collect()
    }

    /// Get indicators by threat type
    pub fn get_indicators_by_threat_type(&self, threat_type: &str) -> Vec<&ThreatIndicator> {
        self.indicators.values()
            .filter(|indicator| indicator.threat_type == threat_type)
            .collect()
    }

    /// Add threat source
    pub fn add_source(&mut self, source: ThreatSource) {
        self.sources.push(source);
    }

    /// Get all sources
    pub fn get_sources(&self) -> &[ThreatSource] {
        &self.sources
    }

    /// Load sample threat intelligence data
    pub fn load_sample_data(&mut self) {
        // Sample malicious IPs
        self.add_indicator(ThreatIndicator {
            id: "malicious-ip-001".to_string(),
            indicator_type: IndicatorType::IpAddress,
            value: "192.168.1.100".to_string(),
            threat_type: "malware_c2".to_string(),
            severity: "high".to_string(),
            sources: vec!["sample_feed".to_string()],
            first_seen: 1640995200, // 2022-01-01
            last_seen: 1672531200,  // 2023-01-01
            tags: vec!["c2".to_string(), "malware".to_string()],
        });

        self.add_indicator(ThreatIndicator {
            id: "malicious-ip-002".to_string(),
            indicator_type: IndicatorType::IpAddress,
            value: "10.0.0.50".to_string(),
            threat_type: "phishing".to_string(),
            severity: "medium".to_string(),
            sources: vec!["sample_feed".to_string()],
            first_seen: 1643673600, // 2022-02-01
            last_seen: 1675209600,  // 2023-02-01
            tags: vec!["phishing".to_string()],
        });

        // Sample malicious domains
        self.add_indicator(ThreatIndicator {
            id: "malicious-domain-001".to_string(),
            indicator_type: IndicatorType::Domain,
            value: "malicious-site.example.com".to_string(),
            threat_type: "phishing".to_string(),
            severity: "high".to_string(),
            sources: vec!["sample_feed".to_string()],
            first_seen: 1640995200,
            last_seen: 1672531200,
            tags: vec!["phishing".to_string(), "fake_login".to_string()],
        });

        // Add sample source
        self.add_source(ThreatSource {
            name: "Sample Threat Feed".to_string(),
            url: "https://example.com/threat-feed".to_string(),
            last_updated: 1672531200,
            confidence: 0.8,
        });
    }
}

/// Threat intelligence processor
pub struct ThreatProcessor {
    feed: ThreatFeed,
}

impl ThreatProcessor {
    pub fn new() -> Self {
        let mut processor = Self {
            feed: ThreatFeed::new(),
        };
        processor.feed.load_sample_data();
        processor
    }

    /// Process cyber event against threat intelligence
    pub fn process_event(&self, event_value: &str, indicator_type: IndicatorType) -> Option<String> {
        if let Some(indicator) = self.feed.is_threat(event_value, indicator_type) {
            Some(format!("Threat detected: {} ({})", indicator.threat_type, indicator.severity))
        } else {
            None
        }
    }

    /// Get threat statistics
    pub fn get_statistics(&self) -> HashMap<String, usize> {
        let mut stats = HashMap::new();

        stats.insert("total_indicators".to_string(), self.feed.indicators.len());
        stats.insert("ip_indicators".to_string(), self.feed.get_indicators_by_type(IndicatorType::IpAddress).len());
        stats.insert("domain_indicators".to_string(), self.feed.get_indicators_by_type(IndicatorType::Domain).len());
        stats.insert("sources".to_string(), self.feed.sources.len());

        stats
    }

    /// Export threat indicators as JSON
    pub fn export_indicators(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(&self.feed.indicators)
    }

    /// Import threat indicators from JSON
    pub fn import_indicators(&mut self, json_data: &str) -> Result<(), serde_json::Error> {
        let indicators: HashMap<String, ThreatIndicator> = serde_json::from_str(json_data)?;
        self.feed.indicators.extend(indicators);
        Ok(())
    }
}

impl Default for ThreatProcessor {
    fn default() -> Self {
        Self::new()
    }
}
