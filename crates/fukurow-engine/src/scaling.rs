//! # Horizontal Scaling
//!
//! Distributed processing and load balancing capabilities

use crate::{ReasoningEngine, ProcessingOptions, EngineResult, EngineError};
use async_trait::async_trait;
use fukurow_core::model::{CyberEvent, SecurityAction};
use fukurow_store::store::RdfStore;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn, error};
use uuid::Uuid;

/// Scaling configuration
#[derive(Debug, Clone)]
pub struct ScalingConfig {
    /// Number of worker instances
    pub num_workers: usize,

    /// Maximum queue size per worker
    pub max_queue_size: usize,

    /// Load balancing strategy
    pub load_balancer: LoadBalancerType,

    /// Health check interval in seconds
    pub health_check_interval_seconds: u64,

    /// Worker timeout in seconds
    pub worker_timeout_seconds: u64,
}

/// Load balancer type
#[derive(Debug, Clone)]
pub enum LoadBalancerType {
    /// Round-robin distribution
    RoundRobin,

    /// Least-loaded worker first
    LeastLoaded,

    /// Hash-based distribution (sticky sessions)
    HashBased,

    /// Adaptive load balancing
    Adaptive,
}

/// Worker instance
struct WorkerInstance {
    id: Uuid,
    queue_size: usize,
    last_health_check: std::time::Instant,
    is_healthy: bool,
}

/// Distributed reasoning engine
pub struct DistributedReasoningEngine {
    config: ScalingConfig,
    workers: Vec<Arc<RwLock<WorkerInstance>>>,
    worker_channels: Vec<mpsc::UnboundedSender<WorkItem>>,
    load_balancer: Box<dyn LoadBalancer>,
    next_worker_index: usize,
}

impl DistributedReasoningEngine {
    /// Create a new distributed reasoning engine
    pub fn new(config: ScalingConfig) -> Self {
        let mut workers = Vec::new();
        let mut worker_channels = Vec::new();

        for _ in 0..config.num_workers {
            let worker_id = Uuid::new_v4();
            let (tx, rx) = mpsc::unbounded_channel();

            let worker = Arc::new(RwLock::new(WorkerInstance {
                id: worker_id,
                queue_size: 0,
                last_health_check: std::time::Instant::now(),
                is_healthy: true,
            }));

            workers.push(Arc::clone(&worker));
            worker_channels.push(tx);

            // Start worker task
            Self::start_worker(worker, rx, config.worker_timeout_seconds);
        }

        let load_balancer: Box<dyn LoadBalancer> = match config.load_balancer {
            LoadBalancerType::RoundRobin => Box::new(RoundRobinBalancer::new(config.num_workers)),
            LoadBalancerType::LeastLoaded => Box::new(LeastLoadedBalancer::new(workers.clone())),
            LoadBalancerType::HashBased => Box::new(HashBasedBalancer::new(config.num_workers)),
            LoadBalancerType::Adaptive => Box::new(AdaptiveBalancer::new(workers.clone())),
        };

        Self {
            config,
            workers,
            worker_channels,
            load_balancer,
            next_worker_index: 0,
        }
    }

    /// Start worker task
    fn start_worker(
        worker: Arc<RwLock<WorkerInstance>>,
        mut rx: mpsc::UnboundedReceiver<WorkItem>,
        timeout_seconds: u64,
    ) {
        tokio::spawn(async move {
            let timeout = std::time::Duration::from_secs(timeout_seconds);

            while let Some(work_item) = rx.recv().await {
                // Update worker queue size
                {
                    let mut worker_guard = worker.write().await;
                    worker_guard.queue_size = worker_guard.queue_size.saturating_sub(1);
                }

                // Process work item with timeout
                let process_future = Self::process_work_item(work_item);
                match tokio::time::timeout(timeout, process_future).await {
                    Ok(result) => {
                        if let Err(e) = result {
                            error!("Worker processing error: {}", e);
                        }
                    }
                    Err(_) => {
                        warn!("Worker processing timeout");
                        let mut worker_guard = worker.write().await;
                        worker_guard.is_healthy = false;
                    }
                }
            }
        });
    }

    /// Process work item
    async fn process_work_item(work_item: WorkItem) -> Result<(), EngineError> {
        match work_item {
            WorkItem::Reasoning { store, options, response_tx } => {
                let engine = ReasoningEngine::new();
                let result = engine.process(&store).await;
                let _ = response_tx.send(result);
            }
            WorkItem::AddEvent { event, response_tx } => {
                let engine = crate::engine::ReasonerEngine::new();
                let result = engine.add_event(event).await.map_err(|e| EngineError::InternalError(format!("Reasoner error: {:?}", e)));
                let _ = response_tx.send(result);
            }
        }
        Ok(())
    }

    /// Submit reasoning task
    pub async fn submit_reasoning(
        &mut self,
        store: RdfStore,
        options: ProcessingOptions,
    ) -> Result<EngineResult, EngineError> {
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let work_item = WorkItem::Reasoning {
            store,
            options,
            response_tx,
        };

        let worker_index = self.load_balancer.select_worker(&work_item)?;
        self.submit_to_worker(worker_index, work_item).await?;

        // Wait for response
        match response_rx.recv().await {
            Some(result) => result,
            None => Err(EngineError::InternalError("Worker response channel closed".to_string())),
        }
    }

    /// Submit event addition task
    pub async fn submit_event(&mut self, event: CyberEvent) -> Result<(), EngineError> {
        let (response_tx, mut response_rx) = mpsc::unbounded_channel();

        let work_item = WorkItem::AddEvent {
            event,
            response_tx,
        };

        let worker_index = self.load_balancer.select_worker(&work_item)?;
        self.submit_to_worker(worker_index, work_item).await?;

        // Wait for response
        match response_rx.recv().await {
            Some(result) => result,
            None => Err(EngineError::InternalError("Worker response channel closed".to_string())),
        }
    }

    /// Submit work to specific worker
    async fn submit_to_worker(&mut self, worker_index: usize, work_item: WorkItem) -> Result<(), EngineError> {
        if worker_index >= self.worker_channels.len() {
            return Err(EngineError::InternalError("Invalid worker index".to_string()));
        }

        // Update worker queue size
        {
            let worker = &self.workers[worker_index];
            let mut worker_guard = worker.write().await;
            worker_guard.queue_size += 1;

            if worker_guard.queue_size > self.config.max_queue_size {
                return Err(EngineError::InternalError("Worker queue full".to_string()));
            }
        }

        self.worker_channels[worker_index]
            .send(work_item)
            .map_err(|_| EngineError::InternalError("Failed to send work to worker".to_string()))
    }

    /// Get scaling metrics
    pub async fn get_metrics(&self) -> ScalingMetrics {
        let mut worker_metrics = Vec::new();

        for worker in &self.workers {
            let worker_guard = worker.read().await;
            worker_metrics.push(WorkerMetrics {
                id: worker_guard.id,
                queue_size: worker_guard.queue_size,
                is_healthy: worker_guard.is_healthy,
                last_health_check: worker_guard.last_health_check.elapsed(),
            });
        }

        ScalingMetrics {
            total_workers: self.config.num_workers,
            active_workers: worker_metrics.iter().filter(|w| w.is_healthy).count(),
            total_queue_size: worker_metrics.iter().map(|w| w.queue_size).sum(),
            worker_metrics,
        }
    }

    /// Health check
    pub async fn health_check(&self) -> bool {
        for worker in &self.workers {
            let worker_guard = worker.read().await;
            if !worker_guard.is_healthy {
                return false;
            }
        }
        true
    }
}

/// Work item for processing
enum WorkItem {
    Reasoning {
        store: RdfStore,
        options: ProcessingOptions,
        response_tx: mpsc::UnboundedSender<Result<EngineResult, EngineError>>,
    },
    AddEvent {
        event: CyberEvent,
        response_tx: mpsc::UnboundedSender<Result<(), EngineError>>,
    },
}

/// Load balancer trait
#[async_trait]
trait LoadBalancer: Send + Sync {
    fn select_worker(&mut self, work_item: &WorkItem) -> Result<usize, EngineError>;
}

/// Round-robin load balancer
struct RoundRobinBalancer {
    num_workers: usize,
    next_worker: usize,
}

impl RoundRobinBalancer {
    fn new(num_workers: usize) -> Self {
        Self {
            num_workers,
            next_worker: 0,
        }
    }
}

#[async_trait]
impl LoadBalancer for RoundRobinBalancer {
    fn select_worker(&mut self, _work_item: &WorkItem) -> Result<usize, EngineError> {
        let worker_index = self.next_worker;
        self.next_worker = (self.next_worker + 1) % self.num_workers;
        Ok(worker_index)
    }
}

/// Least-loaded load balancer
struct LeastLoadedBalancer {
    workers: Vec<Arc<RwLock<WorkerInstance>>>,
}

impl LeastLoadedBalancer {
    fn new(workers: Vec<Arc<RwLock<WorkerInstance>>>) -> Self {
        Self { workers }
    }
}

#[async_trait]
impl LoadBalancer for LeastLoadedBalancer {
    fn select_worker(&mut self, _work_item: &WorkItem) -> Result<usize, EngineError> {
        let mut min_load = usize::MAX;
        let mut selected_worker = 0;

        for (index, worker) in self.workers.iter().enumerate() {
            let worker_guard = worker.try_read().map_err(|_| {
                EngineError::InternalError("Failed to read worker state".to_string())
            })?;

            if worker_guard.is_healthy && worker_guard.queue_size < min_load {
                min_load = worker_guard.queue_size;
                selected_worker = index;
            }
        }

        Ok(selected_worker)
    }
}

/// Hash-based load balancer
struct HashBasedBalancer {
    num_workers: usize,
}

impl HashBasedBalancer {
    fn new(num_workers: usize) -> Self {
        Self { num_workers }
    }
}

#[async_trait]
impl LoadBalancer for HashBasedBalancer {
    fn select_worker(&mut self, work_item: &WorkItem) -> Result<usize, EngineError> {
        let hash = match work_item {
            WorkItem::Reasoning { store, .. } => {
                // Hash based on store content
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                // Hash the number of triples instead of the map itself
                store.statistics().total_triples.hash(&mut hasher);
                hasher.finish()
            }
            WorkItem::AddEvent { event, .. } => {
                // Hash based on event type and timestamp
                use std::collections::hash_map::DefaultHasher;
                use std::hash::{Hash, Hasher};
                let mut hasher = DefaultHasher::new();
                match event {
                    fukurow_core::model::CyberEvent::NetworkConnection { timestamp, .. } => {
                        "network".hash(&mut hasher);
                        timestamp.hash(&mut hasher);
                    }
                    fukurow_core::model::CyberEvent::FileAccess { timestamp, .. } => {
                        "file".hash(&mut hasher);
                        timestamp.hash(&mut hasher);
                    }
                    fukurow_core::model::CyberEvent::ProcessExecution { timestamp, .. } => {
                        "process".hash(&mut hasher);
                        timestamp.hash(&mut hasher);
                    }
                    fukurow_core::model::CyberEvent::UserLogin { timestamp, .. } => {
                        "login".hash(&mut hasher);
                        timestamp.hash(&mut hasher);
                    }
                }
                hasher.finish()
            }
        };

        Ok((hash % self.num_workers as u64) as usize)
    }
}

/// Adaptive load balancer
struct AdaptiveBalancer {
    workers: Vec<Arc<RwLock<WorkerInstance>>>,
    worker_scores: HashMap<Uuid, f64>,
}

impl AdaptiveBalancer {
    fn new(workers: Vec<Arc<RwLock<WorkerInstance>>>) -> Self {
        let worker_scores = workers.iter()
            .map(|w| {
                let id = futures::executor::block_on(w.read()).id;
                (id, 1.0) // Initial score of 1.0
            })
            .collect();

        Self {
            workers,
            worker_scores,
        }
    }
}

#[async_trait]
impl LoadBalancer for AdaptiveBalancer {
    fn select_worker(&mut self, _work_item: &WorkItem) -> Result<usize, EngineError> {
        let mut best_score = 0.0;
        let mut selected_worker = 0;

        for (index, worker) in self.workers.iter().enumerate() {
            let worker_guard = worker.try_read().map_err(|_| {
                EngineError::InternalError("Failed to read worker state".to_string())
            })?;

            if !worker_guard.is_healthy {
                continue;
            }

            let score = self.worker_scores.get(&worker_guard.id).copied().unwrap_or(1.0);
            let adjusted_score = score / (worker_guard.queue_size as f64 + 1.0);

            if adjusted_score > best_score {
                best_score = adjusted_score;
                selected_worker = index;
            }
        }

        // Update worker score based on selection
        if let Some(worker) = self.workers.get(selected_worker) {
            let worker_id = futures::executor::block_on(worker.read()).id;
            let current_score = self.worker_scores.get(&worker_id).copied().unwrap_or(1.0);
            self.worker_scores.insert(worker_id, current_score * 0.95); // Slight decay
        }

        Ok(selected_worker)
    }
}

/// Scaling metrics
#[derive(Debug, Clone)]
pub struct ScalingMetrics {
    pub total_workers: usize,
    pub active_workers: usize,
    pub total_queue_size: usize,
    pub worker_metrics: Vec<WorkerMetrics>,
}

/// Worker metrics
#[derive(Debug, Clone)]
pub struct WorkerMetrics {
    pub id: Uuid,
    pub queue_size: usize,
    pub is_healthy: bool,
    pub last_health_check: std::time::Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scaling_config() {
        let config = ScalingConfig {
            num_workers: 4,
            max_queue_size: 100,
            load_balancer: LoadBalancerType::RoundRobin,
            health_check_interval_seconds: 30,
            worker_timeout_seconds: 60,
        };

        assert_eq!(config.num_workers, 4);
        assert_eq!(config.max_queue_size, 100);
    }

    #[test]
    fn test_round_robin_balancer() {
        let mut balancer = RoundRobinBalancer::new(3);

        assert_eq!(balancer.select_worker(&WorkItem::AddEvent {
            event: CyberEvent::NetworkConnection {
                source_ip: "test".to_string(),
                dest_ip: "test".to_string(),
                port: 80,
                protocol: "tcp".to_string(),
                timestamp: 0,
            },
            response_tx: mpsc::unbounded_channel().0,
        }).unwrap(), 0);

        assert_eq!(balancer.select_worker(&WorkItem::AddEvent {
            event: CyberEvent::NetworkConnection {
                source_ip: "test".to_string(),
                dest_ip: "test".to_string(),
                port: 80,
                protocol: "tcp".to_string(),
                timestamp: 0,
            },
            response_tx: mpsc::unbounded_channel().0,
        }).unwrap(), 1);

        assert_eq!(balancer.select_worker(&WorkItem::AddEvent {
            event: CyberEvent::NetworkConnection {
                source_ip: "test".to_string(),
                dest_ip: "test".to_string(),
                port: 80,
                protocol: "tcp".to_string(),
                timestamp: 0,
            },
            response_tx: mpsc::unbounded_channel().0,
        }).unwrap(), 2);

        assert_eq!(balancer.select_worker(&WorkItem::AddEvent {
            event: CyberEvent::NetworkConnection {
                source_ip: "test".to_string(),
                dest_ip: "test".to_string(),
                port: 80,
                protocol: "tcp".to_string(),
                timestamp: 0,
            },
            response_tx: mpsc::unbounded_channel().0,
        }).unwrap(), 0);
    }

    #[tokio::test]
    async fn test_distributed_engine_creation() {
        let config = ScalingConfig {
            num_workers: 2,
            max_queue_size: 10,
            load_balancer: LoadBalancerType::RoundRobin,
            health_check_interval_seconds: 30,
            worker_timeout_seconds: 60,
        };

        let engine = DistributedReasoningEngine::new(config);
        assert_eq!(engine.workers.len(), 2);

        let metrics = engine.get_metrics().await;
        assert_eq!(metrics.total_workers, 2);
        assert_eq!(metrics.worker_metrics.len(), 2);
    }
}
