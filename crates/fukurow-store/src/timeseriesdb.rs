//! # Time Series Database Integration
//!
//! 分散時系列データベースとの統合
//! TimescaleDB, InfluxDB, ClickHouse などの時系列DBをサポート

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use async_trait::async_trait;
use crate::{RdfStore, Provenance, GraphId};
use fukurow_core::model::Triple;

#[cfg(feature = "timescaledb")]
use sqlx::PgPool;

#[cfg(feature = "influxdb")]
use influxdb2::Client;

#[cfg(feature = "clickhouse")]
use clickhouse::Client as ChClient;

/// 時系列データポイント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub metric: String,
    pub value: f64,
    pub tags: HashMap<String, String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// 時系列データベース設定
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesConfig {
    pub database_type: TimeSeriesDbType,
    pub connection_string: String,
    pub database_name: Option<String>,
    pub retention_policy: Option<String>,
    pub batch_size: usize,
    pub flush_interval_seconds: u64,
}

/// 時系列データベースタイプ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TimeSeriesDbType {
    TimescaleDB,
    InfluxDB,
    ClickHouse,
}

/// 時系列データベースクライアントトレイト
#[async_trait]
pub trait TimeSeriesClient: Send + Sync {
    async fn connect(&mut self, config: &TimeSeriesConfig) -> Result<(), TimeSeriesError>;
    async fn write_point(&self, point: TimeSeriesPoint) -> Result<(), TimeSeriesError>;
    async fn write_batch(&self, points: Vec<TimeSeriesPoint>) -> Result<(), TimeSeriesError>;
    async fn query_range(&self, metric: &str, start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> Result<Vec<TimeSeriesPoint>, TimeSeriesError>;
    async fn health_check(&self) -> Result<(), TimeSeriesError>;
}

/// TimescaleDBクライアント
#[cfg(feature = "timescaledb")]
pub struct TimescaleClient {
    pool: Option<PgPool>,
}

#[cfg(feature = "timescaledb")]
impl TimescaleClient {
    pub fn new() -> Self {
        Self { pool: None }
    }
}

#[cfg(feature = "timescaledb")]
#[async_trait]
impl TimeSeriesClient for TimescaleClient {
    async fn connect(&mut self, config: &TimeSeriesConfig) -> Result<(), TimeSeriesError> {
        let pool = PgPool::connect(&config.connection_string)
            .await
            .map_err(|e| TimeSeriesError::ConnectionError(e.to_string()))?;

        // Create hypertable if it doesn't exist
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS metrics (
                time TIMESTAMPTZ NOT NULL,
                metric TEXT NOT NULL,
                value DOUBLE PRECISION NOT NULL,
                tags JSONB,
                metadata JSONB
            );

            SELECT create_hypertable('metrics', 'time', if_not_exists => TRUE);
            CREATE INDEX IF NOT EXISTS idx_metrics_metric_time ON metrics (metric, time DESC);
            "#,
        )
        .execute(&pool)
        .await
        .map_err(|e| TimeSeriesError::QueryError(e.to_string()))?;

        self.pool = Some(pool);
        Ok(())
    }

    async fn write_point(&self, point: TimeSeriesPoint) -> Result<(), TimeSeriesError> {
        if let Some(pool) = &self.pool {
            sqlx::query(
                "INSERT INTO metrics (time, metric, value, tags, metadata) VALUES ($1, $2, $3, $4, $5)",
            )
            .bind(point.timestamp)
            .bind(&point.metric)
            .bind(point.value)
            .bind(serde_json::to_value(&point.tags).unwrap_or_default())
            .bind(serde_json::to_value(&point.metadata).unwrap_or_default())
            .execute(pool)
            .await
            .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;

            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn write_batch(&self, points: Vec<TimeSeriesPoint>) -> Result<(), TimeSeriesError> {
        if let Some(pool) = &self.pool {
            let mut tx = pool.begin().await
                .map_err(|e| TimeSeriesError::TransactionError(e.to_string()))?;

            for point in points {
                sqlx::query(
                    "INSERT INTO metrics (time, metric, value, tags, metadata) VALUES ($1, $2, $3, $4, $5)",
                )
                .bind(point.timestamp)
                .bind(&point.metric)
                .bind(point.value)
                .bind(serde_json::to_value(&point.tags).unwrap_or_default())
                .bind(serde_json::to_value(&point.metadata).unwrap_or_default())
                .execute(&mut *tx)
                .await
                .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;
            }

            tx.commit().await
                .map_err(|e| TimeSeriesError::TransactionError(e.to_string()))?;

            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn query_range(&self, metric: &str, start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> Result<Vec<TimeSeriesPoint>, TimeSeriesError> {
        if let Some(pool) = &self.pool {
            let rows = sqlx::query(
                "SELECT time, metric, value, tags, metadata FROM metrics WHERE metric = $1 AND time >= $2 AND time <= $3 ORDER BY time",
            )
            .bind(metric)
            .bind(start)
            .bind(end)
            .fetch_all(pool)
            .await
            .map_err(|e| TimeSeriesError::QueryError(e.to_string()))?;

            let mut points = Vec::new();
            for row in rows {
                let timestamp: chrono::DateTime<chrono::Utc> = row.get(0);
                let metric_name: String = row.get(1);
                let value: f64 = row.get(2);
                let tags: serde_json::Value = row.get(3);
                let metadata: serde_json::Value = row.get(4);

                let tags_map = serde_json::from_value(tags).unwrap_or_default();
                let metadata_map = serde_json::from_value(metadata).unwrap_or_default();

                points.push(TimeSeriesPoint {
                    timestamp,
                    metric: metric_name,
                    value,
                    tags: tags_map,
                    metadata: metadata_map,
                });
            }

            Ok(points)
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn health_check(&self) -> Result<(), TimeSeriesError> {
        if let Some(pool) = &self.pool {
            sqlx::query("SELECT 1")
                .execute(pool)
                .await
                .map_err(|e| TimeSeriesError::HealthCheckError(e.to_string()))?;
            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }
}

/// InfluxDBクライアント
#[cfg(feature = "influxdb")]
pub struct InfluxClient {
    client: Option<Client>,
    bucket: String,
    org: String,
}

#[cfg(feature = "influxdb")]
impl InfluxClient {
    pub fn new(bucket: String, org: String) -> Self {
        Self {
            client: None,
            bucket,
            org,
        }
    }
}

#[cfg(feature = "influxdb")]
#[async_trait]
impl TimeSeriesClient for InfluxClient {
    async fn connect(&mut self, config: &TimeSeriesConfig) -> Result<(), TimeSeriesError> {
        let client = Client::new(&config.connection_string, &config.database_name.unwrap_or_default());
        self.client = Some(client);
        Ok(())
    }

    async fn write_point(&self, point: TimeSeriesPoint) -> Result<(), TimeSeriesError> {
        if let Some(client) = &self.client {
            let mut data_point = influxdb2::models::DataPoint::builder(&point.metric)
                .field("value", point.value);

            for (key, value) in &point.tags {
                data_point = data_point.tag(key, value);
            }

            client.write(&self.bucket, &self.org, vec![data_point])
                .await
                .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;

            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn write_batch(&self, points: Vec<TimeSeriesPoint>) -> Result<(), TimeSeriesError> {
        if let Some(client) = &self.client {
            let data_points: Vec<_> = points.into_iter()
                .map(|point| {
                    let mut dp = influxdb2::models::DataPoint::builder(&point.metric)
                        .field("value", point.value);

                    for (key, value) in &point.tags {
                        dp = dp.tag(key, value);
                    }

                    dp
                })
                .collect();

            client.write(&self.bucket, &self.org, data_points)
                .await
                .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;

            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn query_range(&self, metric: &str, start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> Result<Vec<TimeSeriesPoint>, TimeSeriesError> {
        if let Some(client) = &self.client {
            let query = format!(
                r#"from(bucket: "{}")
                |> range(start: {}, stop: {})
                |> filter(fn: (r) => r._measurement == "{}")"#,
                self.bucket,
                start.to_rfc3339(),
                end.to_rfc3339(),
                metric
            );

            let query_result = client.query::<serde_json::Value>(&query, &self.org)
                .await
                .map_err(|e| TimeSeriesError::QueryError(e.to_string()))?;

            // Parse InfluxDB response (simplified)
            let mut points = Vec::new();

            if let Some(values) = query_result.as_array() {
                for value in values {
                    if let (Some(timestamp), Some(val)) = (
                        value.get("_time").and_then(|t| t.as_str()).and_then(|t| chrono::DateTime::parse_from_rfc3339(t).ok()),
                        value.get("_value").and_then(|v| v.as_f64())
                    ) {
                        points.push(TimeSeriesPoint {
                            timestamp: timestamp.with_timezone(&chrono::Utc),
                            metric: metric.to_string(),
                            value: val,
                            tags: HashMap::new(),
                            metadata: HashMap::new(),
                        });
                    }
                }
            }

            Ok(points)
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn health_check(&self) -> Result<(), TimeSeriesError> {
        if let Some(client) = &self.client {
            client.health().await
                .map_err(|e| TimeSeriesError::HealthCheckError(e.to_string()))?;
            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }
}

/// ClickHouseクライアント
#[cfg(feature = "clickhouse")]
pub struct ClickHouseClient {
    client: Option<ChClient>,
    database: String,
}

#[cfg(feature = "clickhouse")]
impl ClickHouseClient {
    pub fn new(database: String) -> Self {
        Self {
            client: None,
            database,
        }
    }
}

#[cfg(feature = "clickhouse")]
#[async_trait]
impl TimeSeriesClient for ClickHouseClient {
    async fn connect(&mut self, config: &TimeSeriesConfig) -> Result<(), TimeSeriesError> {
        let client = ChClient::default()
            .with_url(&config.connection_string)
            .with_database(&self.database);

        // Create table if it doesn't exist
        client.query(
            r#"
            CREATE TABLE IF NOT EXISTS metrics (
                timestamp DateTime64(3, 'UTC'),
                metric String,
                value Float64,
                tags Map(String, String),
                metadata String
            ) ENGINE = MergeTree()
            ORDER BY (metric, timestamp)
            SETTINGS index_granularity = 8192
            "#,
        )
        .execute()
        .await
        .map_err(|e| TimeSeriesError::ConnectionError(e.to_string()))?;

        self.client = Some(client);
        Ok(())
    }

    async fn write_point(&self, point: TimeSeriesPoint) -> Result<(), TimeSeriesError> {
        if let Some(client) = &self.client {
            client.query("INSERT INTO metrics (timestamp, metric, value, tags, metadata) VALUES (?, ?, ?, ?, ?)")
                .bind(point.timestamp.timestamp_millis() as u64)
                .bind(&point.metric)
                .bind(point.value)
                .bind(&point.tags)
                .bind(&serde_json::to_string(&point.metadata).unwrap_or_default())
                .execute()
                .await
                .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;

            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn write_batch(&self, points: Vec<TimeSeriesPoint>) -> Result<(), TimeSeriesError> {
        if let Some(client) = &self.client {
            let mut insert = client.insert("metrics")?;

            for point in points {
                insert.write(&(
                    point.timestamp.timestamp_millis() as u64,
                    point.metric,
                    point.value,
                    point.tags,
                    serde_json::to_string(&point.metadata).unwrap_or_default(),
                )).await
                .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;
            }

            insert.end().await
                .map_err(|e| TimeSeriesError::WriteError(e.to_string()))?;

            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn query_range(&self, metric: &str, start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> Result<Vec<TimeSeriesPoint>, TimeSeriesError> {
        if let Some(client) = &self.client {
            let rows = client.query(&format!(
                "SELECT timestamp, metric, value, tags, metadata FROM metrics WHERE metric = ? AND timestamp >= ? AND timestamp <= ? ORDER BY timestamp",
            ))
            .bind(metric)
            .bind(start.timestamp_millis() as u64)
            .bind(end.timestamp_millis() as u64)
            .fetch_all::<(u64, String, f64, HashMap<String, String>, String)>()
            .await
            .map_err(|e| TimeSeriesError::QueryError(e.to_string()))?;

            let mut points = Vec::new();
            for (timestamp_ms, metric_name, value, tags, metadata_str) in rows {
                let timestamp = chrono::DateTime::from_timestamp_millis(timestamp_ms as i64)
                    .unwrap_or_else(|| chrono::Utc::now());

                let metadata: HashMap<String, serde_json::Value> = serde_json::from_str(&metadata_str)
                    .unwrap_or_default();

                points.push(TimeSeriesPoint {
                    timestamp,
                    metric: metric_name,
                    value,
                    tags,
                    metadata,
                });
            }

            Ok(points)
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }

    async fn health_check(&self) -> Result<(), TimeSeriesError> {
        if let Some(client) = &self.client {
            client.query("SELECT 1")
                .execute()
                .await
                .map_err(|e| TimeSeriesError::HealthCheckError(e.to_string()))?;
            Ok(())
        } else {
            Err(TimeSeriesError::NotConnected)
        }
    }
}

/// 統合時系列データベースマネージャー
pub struct TimeSeriesManager {
    config: TimeSeriesConfig,
    client: Box<dyn TimeSeriesClient>,
    buffer: Vec<TimeSeriesPoint>,
    buffer_size: usize,
}

impl TimeSeriesManager {
    pub async fn new(config: TimeSeriesConfig) -> Result<Self, TimeSeriesError> {
        let client: Box<dyn TimeSeriesClient> = match config.database_type {
            #[cfg(feature = "timescaledb")]
            TimeSeriesDbType::TimescaleDB => Box::new(TimescaleClient::new()),
            #[cfg(feature = "influxdb")]
            TimeSeriesDbType::InfluxDB => {
                let bucket = "fukurow".to_string(); // Default bucket
                let org = "fukurow".to_string(); // Default org
                Box::new(InfluxClient::new(bucket, org))
            }
            #[cfg(feature = "clickhouse")]
            TimeSeriesDbType::ClickHouse => {
                let database = config.database_name.clone().unwrap_or_else(|| "default".to_string());
                Box::new(ClickHouseClient::new(database))
            }
            #[cfg(not(any(feature = "timescaledb", feature = "influxdb", feature = "clickhouse")))]
            _ => return Err(TimeSeriesError::UnsupportedDatabase),
        };

        let buffer_size = config.batch_size;

        let mut manager = Self {
            config,
            client,
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
        };

        // Connect to database
        manager.client.connect(&manager.config).await?;

        Ok(manager)
    }

    /// メトリクスを書き込む
    pub async fn write_metric(&mut self, metric: String, value: f64, tags: HashMap<String, String>) -> Result<(), TimeSeriesError> {
        let point = TimeSeriesPoint {
            timestamp: chrono::Utc::now(),
            metric,
            value,
            tags,
            metadata: HashMap::new(),
        };

        self.buffer.push(point);

        // Bufferが満杯になったら書き込み
        if self.buffer.len() >= self.buffer_size {
            self.flush().await?;
        }

        Ok(())
    }

    /// RDFトリプルの変更を時系列データとして記録
    pub async fn record_triple_change(&mut self, triple: &Triple, operation: &str, graph_id: &GraphId, provenance: &Provenance) -> Result<(), TimeSeriesError> {
        let tags = HashMap::from([
            ("subject".to_string(), triple.subject.clone()),
            ("predicate".to_string(), triple.predicate.clone()),
            ("object".to_string(), triple.object.clone()),
            ("operation".to_string(), operation.to_string()),
            ("graph_id".to_string(), graph_id.to_string()),
            ("provenance_type".to_string(), format!("{:?}", provenance)),
        ]);

        self.write_metric("triple_changes".to_string(), 1.0, tags).await
    }

    /// 推論実行時間を記録
    pub async fn record_inference_time(&mut self, duration_ms: f64, rule_name: &str) -> Result<(), TimeSeriesError> {
        let tags = HashMap::from([
            ("rule_name".to_string(), rule_name.to_string()),
        ]);

        self.write_metric("inference_duration_ms".to_string(), duration_ms, tags).await
    }

    /// 異常検知結果を記録
    pub async fn record_anomaly(&mut self, anomaly_score: f64, detector_type: &str, metric_name: &str) -> Result<(), TimeSeriesError> {
        let tags = HashMap::from([
            ("detector_type".to_string(), detector_type.to_string()),
            ("metric_name".to_string(), metric_name.to_string()),
        ]);

        self.write_metric("anomaly_score".to_string(), anomaly_score, tags).await
    }

    /// バッファをフラッシュ
    pub async fn flush(&mut self) -> Result<(), TimeSeriesError> {
        if self.buffer.is_empty() {
            return Ok(());
        }

        let points = std::mem::take(&mut self.buffer);
        self.client.write_batch(points).await
    }

    /// ヘルスチェック
    pub async fn health_check(&self) -> Result<(), TimeSeriesError> {
        self.client.health_check().await
    }

    /// メトリクスをクエリ
    pub async fn query_metric(&self, metric: &str, hours: i64) -> Result<Vec<TimeSeriesPoint>, TimeSeriesError> {
        let end = chrono::Utc::now();
        let start = end - chrono::Duration::hours(hours);

        self.client.query_range(metric, start, end).await
    }
}

/// 時系列データベースエラー
#[derive(Debug, thiserror::Error)]
pub enum TimeSeriesError {
    #[error("Database not connected")]
    NotConnected,

    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("Query error: {0}")]
    QueryError(String),

    #[error("Write error: {0}")]
    WriteError(String),

    #[error("Transaction error: {0}")]
    TransactionError(String),

    #[error("Health check error: {0}")]
    HealthCheckError(String),

    #[error("Unsupported database type")]
    UnsupportedDatabase,

    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timeseries_point_creation() {
        let point = TimeSeriesPoint {
            timestamp: chrono::Utc::now(),
            metric: "test_metric".to_string(),
            value: 42.0,
            tags: HashMap::from([("key".to_string(), "value".to_string())]),
            metadata: HashMap::new(),
        };

        assert_eq!(point.metric, "test_metric");
        assert_eq!(point.value, 42.0);
        assert_eq!(point.tags.get("key"), Some(&"value".to_string()));
    }

    #[test]
    fn test_timeseries_config() {
        let config = TimeSeriesConfig {
            database_type: TimeSeriesDbType::TimescaleDB,
            connection_string: "postgresql://user:pass@localhost/db".to_string(),
            database_name: Some("fukurow".to_string()),
            retention_policy: Some("30d".to_string()),
            batch_size: 100,
            flush_interval_seconds: 60,
        };

        assert_eq!(config.batch_size, 100);
        assert_eq!(config.flush_interval_seconds, 60);
    }

    #[cfg(feature = "timescaledb")]
    #[tokio::test]
    async fn test_timescale_client_creation() {
        let mut client = TimescaleClient::new();
        // Note: This test requires a running PostgreSQL instance with TimescaleDB
        // For CI, we just test that the client can be created
        assert!(client.pool.is_none());
    }
}
