# Reasoner TS - JSON-LD Cyber Security Reasoner

[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](LICENSE)

A high-performance JSON-LD reasoner specialized for cyber security event analysis, built in Rust with WebAssembly support.

## Overview

This project implements a sophisticated reasoning engine that processes cyber security events (EDR/SIEM data) using JSON-LD graphs and OWL-like inference rules. The system is designed with a clean architecture separating concerns across multiple crates:

- **graph**: JSON-LD graph storage and querying
- **reasoner**: Core inference engine with rule evaluation
- **rules-cyber**: Cyber security specific detection rules
- **api**: RESTful web API
- **cli**: Command-line interface

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

### Installation
```bash
git clone https://github.com/gftdcojp/reasoner-ts
cd reasoner-ts
cargo build --release
```

### CLI Usage
```bash
# Start API server
cargo run --bin reasoner-cli -- serve

# Analyze single event
cargo run --bin reasoner-cli -- analyze --json '{"type": "NetworkConnection", "source_ip": "192.168.1.10", "dest_ip": "malicious.example.com"}'

# Process events from file
cargo run --bin reasoner-cli -- process --input events.json --output results.json

# Interactive mode
cargo run --bin reasoner-cli
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
          │  Reasoner Core      │
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

## Project Structure

```
crates/
├── graph/          # JSON-LD graph operations
├── reasoner/       # Inference engine core
├── rules-cyber/    # Cyber security rules
├── api/            # REST API server
└── cli/            # Command-line interface

story.jsonnet       # Process network definition
```

## Development

### Building
```bash
# Debug build
cargo build

# Release build
cargo build --release

# Build specific crate
cargo build -p reasoner-cli
```

### Testing
```bash
# Run all tests
cargo test

# Run tests for specific crate
cargo test -p reasoner-core

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
