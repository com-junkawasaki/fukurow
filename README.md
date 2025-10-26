# 🦉 Fukurow - OWL Reasoning Stack in Rust

<p align="center">
  <img src="assets/026.png" alt="Fukurow Logo" width="200">
</p>

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)
[![OWL Support](https://img.shields.io/badge/OWL-Support_50%25-yellow)](#owl-support)
[![SPARQL](https://img.shields.io/badge/SPARQL-1.1-blue)](#sparql-support)
[![SHACL](https://img.shields.io/badge/SHACL-Core-blue)](#shacl-support)

**OWLプロジェクト**: JSON-LD / RDF / OWL / SPARQL / SHACL ベースの知識推論システム。

目的: OWLの意味論をRustで実装し、サイバー防御のための高速推論エンジンと監査可能な知識ストアを提供。

## 📊 プロジェクト完成度評価 (OWLプロジェクト観点)

| コンポーネント | 完成度 | ステータス |
|--------------|--------|-----------|
| **OWL推論** | 50% | RDFS+OWL Lite+OWL DL基本実装完了 |
| **SPARQL 1.1** | 50% | 基本パーサー実装、W3C準拠テスト開始 |
| **SHACL Core** | 65% | 基本制約実装、W3Cスイート統合中 |
| **RDF/JSON-LD** | 80% | 安定運用可 |
| **推論エンジン** | 75% | パイプライン完備、RDFS統合済み |
| **サイバー防御** | 70% | 検出器実装済み |
| **API/CLI** | 70% | 主要機能完備 |
| **運用基盤** | 60% | CI/CD・配布設定済み |

**総合完成度: 71%** | **実運用準備度: 60%**

## 🦉 OWL Support (50%)

OWL (Web Ontology Language) 推論の実装状況:

### ✅ 実装済み機能
- **fukurow-rdfs**: RDFSレベルの推論エンジン
  - rdfs:subClassOf の推移的閉包
  - rdfs:subPropertyOf の推移的閉包
  - rdfs:domain と rdfs:range による型推論
  - rdf:type 推論と階層的型伝播

- **fukurow-lite**: OWL Lite相当推論エンジン
  - テーブルローアルゴリズム実装
  - クラス階層推論 (subsumption reasoning)
  - オントロジー整合性検証
  - RDFストアからのオントロジー読み込み

- **fukurow-dl**: OWL DL基本実装
  - 拡張クラスコンストラクタ (intersectionOf, unionOf, complementOf, oneOf)
  - プロパティ制約 (someValuesFrom, allValuesFrom, hasValue, cardinality)
  - 拡張テーブルローアルゴリズム (∃-rule, ∀-rule)
  - 個体レベルの推論 (sameAs, differentFrom)

### 🚧 開発中
- OWL DL完全実装 (個体分類、実現化、計算量最適化)
- 停止性保証と終了条件
- 大規模オントロジーテスト (10k+ axioms)

### 計画中のOWL実装
- **fukurow-lite**: OWL Lite相当の推論 (テーブルローアルゴリズム)
- **fukurow-dl**: OWL DL相当の完全推論

### 現状
- OWL語彙の認識: ✅ (RDF/XML, Turtle, JSON-LD)
- RDFS完全推論: ✅ (subClassOf, subPropertyOf, domain, range)
- 推論エンジン統合: ✅ (ReasoningEngine に RDFS ステップ追加)

## 🔍 SPARQL Support (50%)

SPARQL 1.1 クエリエンジンの実装状況:

### ✅ 実装済み機能
- **Parser**: SPARQL構文解析 (logos + winnow)
  - SELECT/CONSTRUCT/ASK/DESCRIBEクエリタイプ ✅
  - PREFIX宣言の解析 ✅
  - 変数解析 ✅
- **Algebra**: 論理代数変換 (BGP, JOIN, UNION, FILTER, OPTIONAL)
- **Optimizer**: クエリ最適化 (フィルタプッシュダウン)
- **Evaluator**: 実行エンジン (SELECT, CONSTRUCT, ASK)

### 🚧 開発中/未実装
- WHERE句の完全パース
- プロパティパス (ZeroOrMore, OneOrMore, Alternative)
- 集約関数 (COUNT, SUM, AVG, MIN, MAX)
- ORDER BY / LIMIT / OFFSET
- SERVICE (フェデレーテッドクエリ)

### 🎯 次のステップ
- WHERE句の構文解析実装
- W3C SPARQL 1.1 テストスイート準拠 (syntax-sparql1-5)
- FILTER/OPTIONAL/UNIONの実装

## ✅ SHACL Support (65%)

SHACL Core + SHACL-SPARQL 検証エンジンの実装状況:

### ✅ 実装済み機能
- **ShapesGraph 読み込み**: SHACL形状のRDFからの読み込み (targetClass, property, datatype, class, hasValue)
- **制約検証**: Node Shape / Property Shape の基本制約
- **検証レポート**: 違反結果の構造化レポート

### ✅ サポートするSHACL Core制約
- ターゲット指定: `targetClass`
- Node Shapes: `class`, `datatype`, `hasValue`
- Property Shapes: `minCount`, `maxCount`

### 🚧 開発中/未実装
- SHACL Core 完全制約セット (pattern, minLength, maxLength, etc.)
- SHACL-SPARQL 拡張制約
- Property Path評価
- W3C準拠テストスイート統合 (コンパイル修正中)
- SHACL-SPARQL拡張制約
- W3C SHACLテストスイート完全準拠

## 🦉 Fukurow Unified Crate

Fukurowの全機能を統合したメインcrateです。簡単な導入で全ての機能を活用できます。

```bash
cargo add fukurow
```

```rust
use fukurow::prelude::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut engine = ReasonerEngine::new();

    let event = CyberEvent::NetworkConnection {
        source_ip: "192.168.1.100".to_string(),
        dest_ip: "10.0.0.1".to_string(),
        port: 443,
        protocol: "TCP".to_string(),
        timestamp: chrono::Utc::now().timestamp(),
    };

    engine.add_event(event).await?;
    let actions = engine.reason().await?;

    println!("Generated {} actions", actions.len());
    Ok(())
}
```

## 🧩 モジュラーアーキテクチャ（crates.io）

公開済み crates（v0.1.0）:
- fukurow-core
- fukurow-store
- fukurow-rules
- fukurow-sparql ✨ **NEW**
- fukurow-shacl ✨ **NEW**
- fukurow-engine
- fukurow-domain-cyber
- fukurow-api
- fukurow-cli
- fukurow (統合)

### ソース構成
```
fukurow/                     # 🦉 統合メインcrate
├── fukurow-core            # 📊 RDF/JSON-LDコアデータモデル
├── fukurow-store           # 💾 RDF Store + provenance付きTriple管理
├── fukurow-rules           # 🛡️ ルールトレイトと制約検証(SHACL相当)
├── fukurow-sparql          # 🔍 SPARQL 1.1 クエリエンジン ✨ NEW
├── fukurow-shacl           # ✅ SHACL Core 検証エンジン ✨ NEW
├── fukurow-engine          # 🧠 推論オーケストレーション
├── fukurow-domain-cyber    # 🔒 サイバー防御ドメインルール群
├── fukurow-api             # 🌐 RESTful Web API
└── fukurow-cli             # 💻 コマンドラインインターフェース
```

## ⚙️ fukurow-store: RDF Store設計

### 役割
* 観測事実・推論事実を格納する軽量RDFストア。
* provenance (Sensor/Inferred) と timestamp を管理。
* サイバー防御で必要な監査・トレーサビリティを確保。

### 型モデル
```rust
pub struct StoredTriple {
    pub graph_id: GraphId,
    pub triple: Triple,
    pub asserted_at: Timestamp,
    pub provenance: Provenance,
}

pub enum Provenance {
    Sensor { source: String },
    Inferred { rule: String },
}
```

## Key Features

### 🔍 Advanced Threat Detection
- **Pattern-based detection**: Ransomware, lateral movement, privilege escalation
- **Behavioral analysis**: Anomaly detection with configurable thresholds
- **Threat intelligence integration**: IOC matching against known malicious indicators
- **Rule engine**: Extensible inference rules for custom threat scenarios

### 🏗️ Architecture
- **JSON-LD native**: Semantic web standards for knowledge representation
- **Immutable reasoning**: Side-effect free inference with action proposals only
- **Concurrent processing**: Async/await with Tokio runtime
- **WebAssembly ready**: Future browser deployment support

### 🚀 Performance
- **Zero-copy operations**: Efficient memory usage with Rust ownership model
- **Compiled rules**: Fast pattern matching with optimized data structures
- **Scalable graph storage**: In-memory with future persistent storage options

## Quick Start

### Prerequisites
- Rust 1.70+
- Cargo

### Installation (via crates.io)
```bash
cargo add fukurow
```

### From source
```bash
git clone https://github.com/com-junkawasaki/fukurow
cd fukurow
cargo build --release
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fukurow-core
cargo test -p fukurow-domain-cyber
```

### CLI Usage
```bash
# Start API server
cargo run --bin fukurow-cli -- serve

# Analyze single event
cargo run --bin fukurow-cli -- analyze --json '{"type": "NetworkConnection", "source_ip": "192.168.1.10", "dest_ip": "192.168.1.100"}'

# Process events from file
cargo run --bin fukurow-cli -- process --input events.json --output results.json

# Interactive mode
cargo run --bin fukurow-cli
```

### API Usage
```bash
# Submit event
curl -X POST http://localhost:3000/events \
  -H "Content-Type: application/json" \
  -d '{"event": {"type": "NetworkConnection", "source_ip": "192.168.1.10", "dest_ip": "10.0.0.50"}}'

# Execute reasoning
curl -X POST http://localhost:3000/reason \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Architecture

```
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   CLI Tool      │    │   REST API      │    │   WebAssembly   │
│                 │    │                 │    │   (Future)      │
│ • Interactive   │    │ • JSON/HTTP     │    │                 │
│ • Batch proc.   │    │ • CORS enabled  │    │                 │
│ • File I/O      │    │ • OpenAPI docs  │    └─────────────────┘
└─────────┬───────┘    └─────────┬───────┘
          │                      │
          └──────────┬───────────┘
                     │
          ┌─────────────────────┐
          │  Fukurow Core       │
          │                     │
          │ • Rule Engine       │
          │ • Inference Logic   │
          │ • Action Proposals  │
          └─────────┬───────────┘
                    │
          ┌─────────────────────┐
          │  Cyber Rules        │
          │                     │
          │ • Threat Patterns   │
          │ • Anomaly Detection │
          │ • IOC Matching      │
          └─────────┬───────────┘
                    │
          ┌─────────────────────┐
          │  Graph Storage      │
          │                     │
          │ • JSON-LD triples   │
          │ • SPARQL queries    │
          │ • Semantic indexing │
          └─────────────────────┘
```

## 📚 RDF Store選定方針

| 方式                | 特徴             | 適用領域       |
| ----------------- | -------------- | ---------- |
| Rustネイティブ         | 高速・GCレス・WASM化可 | リアルタイム防御コア |
| RDB (Postgres等)   | 永続・監査性         | 長期監査・履歴分析  |
| 外部トリプルストア (Jena等) | 完全SPARQL・既存資産  | バッチ/夜間監査   |

結論: **fukurow-storeはRust内製インメモリ＋永続サポート**、監査・長期分析は外部連携。

## 🌙 総括

* fukurowは「知識グラフストア × 推論 × 即時アクション × 監査クエリ」の統合基盤。
* JSON-LDをI/Oにし、OWLの意味論をRustルールにコンパイルする。
* 夜中でも眠らず判断するシステムのための、覚醒した知識推論フクロウ。🦉

## Development

### Building
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p fukurow-cli
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p fukurow-core

# Run with coverage (requires tarpaulin)
cargo tarpaulin
```

### Code Quality
```bash
# Format code
cargo fmt

# Lint code
cargo clippy

# Check documentation
cargo doc --open
```

## API Documentation

### Endpoints

- `GET /health` - Health check
- `POST /events` - Submit cyber event
- `POST /reason` - Execute reasoning
- `POST /graph/query` - Query knowledge graph
- `GET /threat-intel` - Threat intelligence info
- `GET /stats` - System statistics

### Event Types

```json
{
  "type": "NetworkConnection",
  "source_ip": "192.168.1.10",
  "dest_ip": "10.0.0.50",
  "port": 443,
  "protocol": "tcp",
  "timestamp": 1640995200
}
```

### Action Types

```json
{
  "action_type": "IsolateHost",
  "parameters": {
    "host_ip": "192.168.1.100",
    "reason": "Malicious activity detected"
  }
}
```

## Configuration

The system is configured via:

1. **Environment variables** for runtime settings
2. **Rule files** for custom inference rules
3. **Threat feeds** for indicator updates
4. **API configuration** for server settings

## Security Considerations

- **No direct execution**: Actions are proposals only
- **Auditable reasoning**: Full inference chain logging
- **Input validation**: Strict JSON-LD schema validation
- **Rate limiting**: Configurable API rate limits
- **Authentication**: JWT-based API authentication (future)

## Performance Characteristics

- **Memory**: O(n) for graph size, efficient triple storage
- **CPU**: Linear rule evaluation, optimized pattern matching
- **Network**: Minimal I/O, efficient JSON-LD serialization
- **Concurrency**: Async processing with Tokio runtime

## 📈 Success Metrics (OWLプロジェクト基準)

### OWL推論品質
- **RDFS準拠**: 規則セットの閉包完全性 (W3C RDFS仕様準拠)
- **OWL Lite準拠**: テーブルロー推論の健全性・完全性
- **OWL DL準拠**: 計算量分析済み・停止性保証

### クエリ・検証品質
- **SPARQL準拠**: W3C SPARQL 1.1 テスト90%+ (主要カテゴリ)
- **SHACL準拠**: W3C SHACLテストスイート90%+
- **RDF準拠**: JSON-LD/Turtle/RDF/XML完全サポート

### パフォーマンス指標
- **推論性能**: 10kトリプルでp50<50ms, p95<150ms
- **クエリ性能**: BGP 3-5パターンで<10ms
- **メモリ効率**: <256MB/10kトリプル

### サイバー防御機能
- **検出精度**: 脅威パターンカバレッジ95%+
- **誤検知率**: <5% (運用データ検証済み)
- **応答時間**: <100ms/APIコール

### 運用品質
- **安定性**: 99.9% uptime, 障害時graceful degradation
- **セキュリティ**: Zero known vulnerabilities, 監査ログ完全性
- **保守性**: テストカバレッジ85%+, ドキュメント完備

## 🛣️ OWLプロジェクト ロードマップ

### Phase 1: 基盤強化 (2-4週間)
- [x] SPARQL 1.1 基本実装 (Parser/Algebra/Optimizer/Evaluator)
- [x] SHACL Core 検証エンジン実装
- [ ] SPARQL W3C準拠テスト (主要カテゴリ90%+)
- [ ] SHACL W3Cテストスイート統合
- [ ] RDFS推論実装 (`fukurow-rdfs`)
- [ ] ストア統計 + 結合順序最適化

### Phase 2: OWL Lite 実装 (4-6週間)
- [ ] OWL Lite相当推論 (`fukurow-lite`)
- [ ] テーブルロー推論アルゴリズム
- [ ] 健全性・停止性検証
- [ ] パフォーマンス最適化 (10kトリプルでp50<50ms)

### Phase 3: OWL DL 拡張 (6-8週間)
- [ ] OWL DL相当完全推論 (`fukurow-dl`)
- [ ] 計算量分析・最適化
- [ ] 大規模オントロジーテスト

### Phase 4: WebAssembly & 分散化 (8-12週間)
- [ ] WebAssembly compilation for browser deployment
  - [ ] Expose `fukurow-core` to `wasm32-unknown-unknown` with `wasm-bindgen`
  - [ ] Add `wasm` feature flags for `fukurow-engine` and `fukurow-store`
  - [ ] Switch `uuid v4`/`getrandom` to `uuid/js` + `getrandom/js`
  - [ ] Replace `chrono::Utc::now()` with `js_sys::Date` or injected clock
  - [ ] Remove Tokio runtime assumptions; use `wasm-bindgen-futures` (`spawn_local`)
  - [ ] Provide `cdylib` exports for reasoning entry points
  - [ ] Minimal browser demo (load WASM, feed event, read actions)
  - [ ] CI job: `wasm32-unknown-unknown` build and size budget check
  - [ ] Benchmarks in Web Worker; document perf trade-offs

- [ ] Vercelでの動作/配信
  - [ ] Astro/静的サイトでWASMデモをホスト（`astoro/` を `vercel build` 対応）
  - [ ] `vercel.json` と Build Output API v3 で静的出力/エッジ関数を定義
  - [ ] Edge Function 経由の軽量APIブリッジ（必要時、WASM呼び出しのラッパ）
  - [ ] Edgeランタイム互換性確認（fs/ネイティブ拡張非依存、Web Crypto採用）
  - [ ] CI: `vercel pull --yes && vercel build --prod` ドライランを追加
  - [ ] バンドルサイズとTTFBのSLO設定（サイズ上限/キャッシュ戦略）

- [ ] Persistent graph storage (PostgreSQL, Neo4j)
- [ ] Distributed reasoning across multiple nodes
- [ ] Real-time streaming event processing

### Phase 5: エンタープライズ対応 (12-16週間)
- [ ] Advanced ML-based anomaly detection
- [ ] Integration with SIEM platforms
- [ ] Rule DSL for custom threat scenarios
- [ ] Enterprise security compliance

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

Dual-licensed under MIT or Apache 2.0.

## Acknowledgments

Built with Rust ecosystem crates including Sophia, Tokio, Axum, and Serde.
