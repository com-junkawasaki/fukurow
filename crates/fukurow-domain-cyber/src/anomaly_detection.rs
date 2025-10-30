//! # ML-Based Anomaly Detection
//!
//! 時系列分析と統計的手法によるセキュリティイベント異常検知
//! シャノン情報論に基づく効率的な異常検知アルゴリズム

use std::collections::VecDeque;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Deserialize, Serialize};

/// 時系列データポイント
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesPoint {
    pub timestamp: u64,
    pub value: f64,
    pub label: String,
}

/// 異常検知結果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnomalyResult {
    pub timestamp: u64,
    pub value: f64,
    pub label: String,
    pub score: f64,
    pub threshold: f64,
    pub is_anomaly: bool,
    pub method: String,
}

/// 統計的異常検知器
#[derive(Debug, Clone)]
pub struct StatisticalDetector {
    window_size: usize,
    z_threshold: f64,
    data: VecDeque<f64>,
    timestamps: VecDeque<u64>,
}

impl StatisticalDetector {
    pub fn new(window_size: usize, z_threshold: f64) -> Self {
        Self {
            window_size,
            z_threshold,
            data: VecDeque::with_capacity(window_size),
            timestamps: VecDeque::with_capacity(window_size),
        }
    }

    pub fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult> {
        // データを追加
        self.data.push_back(point.value);
        self.timestamps.push_back(point.timestamp);

        // ウィンドウサイズを超えたら古いデータを削除
        if self.data.len() > self.window_size {
            self.data.pop_front();
            self.timestamps.pop_front();
        }

        // 十分なデータがない場合は異常検知しない
        if self.data.len() < 10 {
            return None;
        }

        self.detect_anomaly(&point)
    }

    fn detect_anomaly(&self, point: &TimeSeriesPoint) -> Option<AnomalyResult> {
        // 平均と標準偏差を計算
        let (mean, std_dev) = self.calculate_stats()?;

        if std_dev == 0.0 {
            return None;
        }

        // Z-score計算
        let z_score = (point.value - mean).abs() / std_dev;

        let is_anomaly = z_score > self.z_threshold;
        let threshold = mean + self.z_threshold * std_dev;

        Some(AnomalyResult {
            timestamp: point.timestamp,
            value: point.value,
            label: point.label.clone(),
            score: z_score,
            threshold,
            is_anomaly,
            method: "z_score".to_string(),
        })
    }

    fn calculate_stats(&self) -> Option<(f64, f64)> {
        if self.data.is_empty() {
            return None;
        }

        let n = self.data.len() as f64;
        let mean = self.data.iter().sum::<f64>() / n;
        let variance = self.data.iter()
            .map(|x| (x - mean).powi(2))
            .sum::<f64>() / n;
        let std_dev = variance.sqrt();

        Some((mean, std_dev))
    }
}

/// IQR（四分位範囲）ベース異常検知器
#[derive(Debug, Clone)]
pub struct IQRDetector {
    window_size: usize,
    multiplier: f64,
    data: VecDeque<f64>,
}

impl IQRDetector {
    pub fn new(window_size: usize, multiplier: f64) -> Self {
        Self {
            window_size,
            multiplier,
            data: VecDeque::with_capacity(window_size),
        }
    }

    pub fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult> {
        self.data.push_back(point.value);

        if self.data.len() > self.window_size {
            self.data.pop_front();
        }

        if self.data.len() < 10 {
            return None;
        }

        self.detect_anomaly(&point)
    }

    fn detect_anomaly(&self, point: &TimeSeriesPoint) -> Option<AnomalyResult> {
        let mut sorted_data: Vec<f64> = self.data.iter().cloned().collect();
        sorted_data.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

        let q1_idx = (sorted_data.len() as f64 * 0.25) as usize;
        let q3_idx = (sorted_data.len() as f64 * 0.75) as usize;

        let q1 = sorted_data[q1_idx];
        let q3 = sorted_data[q3_idx];
        let iqr = q3 - q1;

        let lower_bound = q1 - self.multiplier * iqr;
        let upper_bound = q3 + self.multiplier * iqr;

        let is_anomaly = point.value < lower_bound || point.value > upper_bound;
        let threshold = if point.value > q3 { upper_bound } else { lower_bound };

        Some(AnomalyResult {
            timestamp: point.timestamp,
            value: point.value,
            label: point.label.clone(),
            score: ((point.value - (q1 + q3) / 2.0) / iqr).abs(),
            threshold,
            is_anomaly,
            method: "iqr".to_string(),
        })
    }
}

/// 指数移動平均ベースのトレンド検知器
#[derive(Debug, Clone)]
pub struct TrendDetector {
    alpha: f64,
    threshold: f64,
    ema: Option<f64>,
    trend_score: f64,
    window_size: usize,
    recent_values: VecDeque<f64>,
}

impl TrendDetector {
    pub fn new(alpha: f64, threshold: f64, window_size: usize) -> Self {
        Self {
            alpha,
            threshold,
            ema: None,
            trend_score: 0.0,
            window_size,
            recent_values: VecDeque::with_capacity(window_size),
        }
    }

    pub fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult> {
        self.recent_values.push_back(point.value);
        if self.recent_values.len() > self.window_size {
            self.recent_values.pop_front();
        }

        // EMA更新
        self.ema = Some(match self.ema {
            None => point.value,
            Some(ema) => self.alpha * point.value + (1.0 - self.alpha) * ema,
        });

        // トレンドスコア計算（最近の値とEMAの差）
        if let Some(ema) = self.ema {
            let recent_avg = self.recent_values.iter().sum::<f64>() / self.recent_values.len() as f64;
            self.trend_score = (recent_avg - ema).abs() / ema.abs().max(1.0);
        }

        let is_anomaly = self.trend_score > self.threshold;

        Some(AnomalyResult {
            timestamp: point.timestamp,
            value: point.value,
            label: point.label.clone(),
            score: self.trend_score,
            threshold: self.threshold,
            is_anomaly,
            method: "trend".to_string(),
        })
    }
}

/// 統合異常検知器マネージャー
#[derive(Debug)]
pub struct AnomalyDetectorManager {
    detectors: std::collections::HashMap<String, Box<dyn AnomalyDetectorTrait>>,
    ensemble_threshold: f64,
}

impl AnomalyDetectorManager {
    pub fn new(ensemble_threshold: f64) -> Self {
        Self {
            detectors: std::collections::HashMap::new(),
            ensemble_threshold,
        }
    }

    pub fn add_detector(&mut self, name: String, detector: Box<dyn AnomalyDetectorTrait>) {
        self.detectors.insert(name, detector);
    }

    pub fn detect_anomalies(&mut self, point: TimeSeriesPoint) -> Vec<AnomalyResult> {
        let mut results = Vec::new();
        let mut anomaly_count = 0;

        // 各検知器で異常検知
        for detector in self.detectors.values_mut() {
            if let Some(result) = detector.add_point(point.clone()) {
                results.push(result.clone());
                if result.is_anomaly {
                    anomaly_count += 1;
                }
            }
        }

        // アンサンブル判定：複数の検知器が異常と判定した場合
        if anomaly_count as f64 >= self.ensemble_threshold * self.detectors.len() as f64 {
            for result in &mut results {
                result.is_anomaly = true;
                result.method = format!("ensemble_{}", result.method);
            }
        }

        results
    }
}

/// 異常検知器トレイト
pub trait AnomalyDetectorTrait: std::fmt::Debug {
    fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult>;
}

// トレイト実装
impl AnomalyDetectorTrait for StatisticalDetector {
    fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult> {
        self.add_point(point)
    }
}

impl AnomalyDetectorTrait for IQRDetector {
    fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult> {
        self.add_point(point)
    }
}

impl AnomalyDetectorTrait for TrendDetector {
    fn add_point(&mut self, point: TimeSeriesPoint) -> Option<AnomalyResult> {
        self.add_point(point)
    }
}

/// セキュリティイベント異常検知器
#[derive(Debug)]
pub struct SecurityAnomalyDetector {
    manager: AnomalyDetectorManager,
    event_counts: std::collections::HashMap<String, u64>,
}

impl SecurityAnomalyDetector {
    pub fn new() -> Self {
        let mut manager = AnomalyDetectorManager::new(0.6); // 60%以上の検知器が異常と判定

        // 統計的検知器（Z-score）
        manager.add_detector(
            "statistical".to_string(),
            Box::new(StatisticalDetector::new(100, 3.0))
        );

        // IQR検知器
        manager.add_detector(
            "iqr".to_string(),
            Box::new(IQRDetector::new(100, 1.5))
        );

        // トレンド検知器
        manager.add_detector(
            "trend".to_string(),
            Box::new(TrendDetector::new(0.1, 0.3, 50))
        );

        Self {
            manager,
            event_counts: std::collections::HashMap::new(),
        }
    }

    pub fn analyze_event(&mut self, event_type: &str, count: u64) -> Vec<AnomalyResult> {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let point = TimeSeriesPoint {
            timestamp,
            value: count as f64,
            label: event_type.to_string(),
        };

        // イベントカウント更新
        *self.event_counts.entry(event_type.to_string()).or_insert(0) = count;

        self.manager.detect_anomalies(point)
    }

    pub fn get_event_statistics(&self) -> std::collections::HashMap<String, u64> {
        self.event_counts.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_statistical_detector_normal() {
        let mut detector = StatisticalDetector::new(10, 2.0);

        // 正常データを追加
        for i in 0..15 {
            let point = TimeSeriesPoint {
                timestamp: i as u64,
                value: 10.0 + (i as f64 * 0.1),
                label: "test".to_string(),
            };
            let result = detector.add_point(point);
            if i >= 10 {
                assert!(result.is_some());
                assert!(!result.unwrap().is_anomaly);
            }
        }
    }

    #[test]
    fn test_statistical_detector_anomaly() {
        let mut detector = StatisticalDetector::new(10, 2.0);

        // 正常データを追加
        for i in 0..10 {
            let point = TimeSeriesPoint {
                timestamp: i as u64,
                value: 10.0,
                label: "test".to_string(),
            };
            detector.add_point(point);
        }

        // 異常データを追加
        let anomaly_point = TimeSeriesPoint {
            timestamp: 10,
            value: 100.0, // 明らかに異常な値
            label: "test".to_string(),
        };

        let result = detector.add_point(anomaly_point).unwrap();
        assert!(result.is_anomaly);
        assert_eq!(result.method, "z_score");
    }

    #[test]
    fn test_security_anomaly_detector() {
        let mut detector = SecurityAnomalyDetector::new();

        // 正常なイベントを追加
        for i in 0..10 {
            let anomalies = detector.analyze_event("login_attempts", 5 + i);
            assert!(anomalies.is_empty() || anomalies.iter().all(|a| !a.is_anomaly));
        }

        // 異常なイベントを追加
        let anomalies = detector.analyze_event("login_attempts", 1000); // 明らかに異常
        assert!(!anomalies.is_empty());

        // アンサンブル判定で異常と判定されるはず
        let has_anomaly = anomalies.iter().any(|a| a.is_anomaly);
        assert!(has_anomaly, "Ensemble detection should catch anomaly");
    }
}
