# 🦉 Fukurow - Rust Reasoning & Knowledge Graph Stack

<p align="center">
  <img src="assets/026.png" alt="Fukurow Logo" width="200">
</p>

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

**JSON-LD / RDF / OWL / SPARQL / GraphQL-LD** ベースの知識を処理する Rust スタック。

目的: 推論・検証・クエリ・アクション提案までを統合し、リアルタイムサイバー防御に利用できる形にする。
高速推論エンジンと監査可能な知識ストアを Rust で統合。

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

## Future Roadmap

- [ ] WebAssembly compilation for browser deployment
- [ ] Persistent graph storage (PostgreSQL, Neo4j)
- [ ] Advanced ML-based anomaly detection
- [ ] Real-time streaming event processing
- [ ] Distributed reasoning across multiple nodes
- [ ] Integration with SIEM platforms
- [ ] Rule DSL for custom threat scenarios

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

## License

Dual-licensed under MIT or Apache 2.0.

## Acknowledgments

Built with Rust ecosystem crates including Sophia, Tokio, Axum, and Serde.
