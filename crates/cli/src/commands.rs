//! CLI command definitions and handlers

use clap::{Parser, Subcommand};
use reasoner_core::ReasonerEngine;
use reasoner_graph::model::CyberEvent;
use rules_cyber::threat_intelligence::{ThreatProcessor, IndicatorType};
use std::path::PathBuf;
use anyhow::Result;

/// Main CLI structure
#[derive(Parser)]
#[command(name = "reasoner")]
#[command(about = "JSON-LD Reasoner for Cyber Security Events")]
#[command(version = env!("CARGO_PKG_VERSION"))]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

/// Available CLI commands
#[derive(Subcommand)]
pub enum Commands {
    /// Start the API server
    Serve {
        /// Host to bind to
        #[arg(long, default_value = "0.0.0.0")]
        host: String,

        /// Port to bind to
        #[arg(short, long, default_value = "3000")]
        port: u16,
    },

    /// Analyze a single event
    Analyze {
        /// Event data as JSON file
        #[arg(short, long)]
        file: Option<PathBuf>,

        /// Event data as JSON string
        #[arg(short, long)]
        json: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Process events from file
    Process {
        /// Input file containing events
        #[arg(short, long)]
        input: PathBuf,

        /// Output file for results
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format
        #[arg(short, long, default_value = "json")]
        format: OutputFormat,
    },

    /// Query the knowledge graph
    Query {
        /// Subject filter
        #[arg(short, long)]
        subject: Option<String>,

        /// Predicate filter
        #[arg(short, long)]
        predicate: Option<String>,

        /// Object filter
        #[arg(short, long)]
        object: Option<String>,

        /// Output format
        #[arg(short, long, default_value = "text")]
        format: OutputFormat,
    },

    /// Threat intelligence operations
    Threat {
        #[command(subcommand)]
        command: ThreatCommands,
    },

    /// Show system information
    Info,
}

/// Threat intelligence subcommands
#[derive(Subcommand)]
pub enum ThreatCommands {
    /// Show threat intelligence statistics
    Stats,

    /// Export threat indicators
    Export {
        /// Output file
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Import threat indicators
    Import {
        /// Input file containing indicators
        #[arg(short, long)]
        input: PathBuf,
    },

    /// Check if value is a known threat
    Check {
        /// Value to check
        #[arg(short, long)]
        value: String,

        /// Type of indicator (ip, domain, hash, etc.)
        #[arg(short, long)]
        r#type: String,
    },
}

/// Output format options
#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
pub enum OutputFormat {
    Text,
    Json,
    JsonPretty,
}

/// Command execution result
pub struct CommandResult {
    pub success: bool,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

/// Execute CLI commands
pub struct CommandExecutor {
    reasoner: ReasonerEngine,
    threat_processor: ThreatProcessor,
}

impl CommandExecutor {
    pub fn new() -> Self {
        let reasoner = ReasonerEngine::new();
        // Note: Adding default rules to reasoner would require API change

        Self {
            reasoner,
            threat_processor: ThreatProcessor::new(),
        }
    }

    /// Execute a CLI command
    pub async fn execute(&mut self, command: Commands) -> Result<CommandResult> {
        match command {
            Commands::Serve { host, port } => self.execute_serve(host, port).await,
            Commands::Analyze { file, json, format } => self.execute_analyze(file, json, format).await,
            Commands::Process { input, output, format } => self.execute_process(input, output, format).await,
            Commands::Query { subject, predicate, object, format } => {
                self.execute_query(subject, predicate, object, format).await
            }
            Commands::Threat { command } => self.execute_threat_command(command).await,
            Commands::Info => self.execute_info(),
        }
    }

    async fn execute_serve(&self, host: String, port: u16) -> Result<CommandResult> {
        use reasoner_api::{ReasonerServer, ServerConfig};

        let config = ServerConfig { host: host.clone(), port, max_connections: 100 };
        let server = ReasonerServer::with_config(config);

        println!("Starting server on {}:{}", host, port);
        println!("Press Ctrl+C to stop");

        server.serve().await?;

        Ok(CommandResult {
            success: true,
            message: "Server stopped".to_string(),
            data: None,
        })
    }

    async fn execute_analyze(
        &mut self,
        file: Option<PathBuf>,
        json: Option<String>,
        format: OutputFormat,
    ) -> Result<CommandResult> {
        let event_data = if let Some(file_path) = file {
            std::fs::read_to_string(file_path)?
        } else if let Some(json_str) = json {
            json_str
        } else {
            return Err(anyhow::anyhow!("Either --file or --json must be specified"));
        };

        let event: CyberEvent = serde_json::from_str(&event_data)?;

        self.reasoner.add_event(event).await?;
        let actions = self.reasoner.reason().await?;

        match format {
            OutputFormat::Text => {
                println!("Event analyzed successfully\nActions proposed: {}", actions.len());
                for (i, action) in actions.iter().enumerate() {
                    println!("{}. {:?}", i + 1, action);
                }
            }
            OutputFormat::Json => println!("{}", serde_json::to_string(&actions)?),
            OutputFormat::JsonPretty => println!("{}", serde_json::to_string_pretty(&actions)?),
        }

        Ok(CommandResult {
            success: true,
            message: "Event analyzed".to_string(),
            data: Some(serde_json::json!({ "actions": actions })),
        })
    }

    async fn execute_process(
        &mut self,
        input: PathBuf,
        output: Option<PathBuf>,
        format: OutputFormat,
    ) -> Result<CommandResult> {
        let content = std::fs::read_to_string(&input)?;
        let events: Vec<CyberEvent> = serde_json::from_str(&content)?;

        let mut all_actions = Vec::new();
        let mut processed_count = 0;

        for event in events {
            self.reasoner.add_event(event).await?;
            processed_count += 1;
        }

        let actions = self.reasoner.reason().await?;
        all_actions.extend(actions);

        let result = match format {
            OutputFormat::Text => {
                format!("Processed {} events, generated {} actions", processed_count, all_actions.len())
            }
            OutputFormat::Json => serde_json::to_string(&all_actions)?,
            OutputFormat::JsonPretty => serde_json::to_string_pretty(&all_actions)?,
        };

        if let Some(output_path) = output {
            std::fs::write(output_path, &result)?;
        } else {
            println!("{}", result);
        }

        Ok(CommandResult {
            success: true,
            message: format!("Processed {} events", processed_count),
            data: Some(serde_json::json!({
                "processed_events": processed_count,
                "actions": all_actions
            })),
        })
    }

    async fn execute_query(
        &self,
        subject: Option<String>,
        predicate: Option<String>,
        object: Option<String>,
        format: OutputFormat,
    ) -> Result<CommandResult> {
        let store = self.reasoner.get_graph_store().await;
        let graph_store = store.read().await;

        let triples = graph_store.find_triples(
            subject.as_deref(),
            predicate.as_deref(),
            object.as_deref(),
        );

        let count = triples.len();
        let result = match format {
            OutputFormat::Text => {
                let mut output = format!("Found {} triples:\n", count);
                for triple in &triples {
                    output.push_str(&format!("  {} {} {}\n", triple.subject, triple.predicate, triple.object));
                }
                output
            }
            OutputFormat::Json => serde_json::to_string(&triples)?,
            OutputFormat::JsonPretty => serde_json::to_string_pretty(&triples)?,
        };

        println!("{}", result);

        Ok(CommandResult {
            success: true,
            message: format!("Found {} triples", count),
            data: Some(serde_json::json!({ "triples": triples })),
        })
    }

    async fn execute_threat_command(&self, command: ThreatCommands) -> Result<CommandResult> {
        match command {
            ThreatCommands::Stats => {
                let stats = self.threat_processor.get_statistics();
                let result = serde_json::to_string_pretty(&stats)?;
                println!("{}", result);

                Ok(CommandResult {
                    success: true,
                    message: "Threat intelligence statistics".to_string(),
                    data: Some(serde_json::json!(stats)),
                })
            }
            ThreatCommands::Export { output } => {
                let json_data = self.threat_processor.export_indicators()?;

                if let Some(output_path) = output {
                    std::fs::write(output_path, &json_data)?;
                } else {
                    println!("{}", json_data);
                }

                Ok(CommandResult {
                    success: true,
                    message: "Threat indicators exported".to_string(),
                    data: Some(serde_json::json!({ "exported": true })),
                })
            }
            ThreatCommands::Import { input } => {
                let _content = std::fs::read_to_string(input)?;
                // Note: Would need mutable access to threat processor
                println!("Import functionality not yet implemented");

                Ok(CommandResult {
                    success: false,
                    message: "Import not implemented".to_string(),
                    data: None,
                })
            }
            ThreatCommands::Check { value, r#type } => {
                // Parse indicator type
                let indicator_type = match r#type.as_str() {
                    "ip" => IndicatorType::IpAddress,
                    "domain" => IndicatorType::Domain,
                    "hash" => IndicatorType::FileHash,
                    _ => return Err(anyhow::anyhow!("Unknown indicator type: {}", r#type)),
                };

                let is_threat = self.threat_processor.process_event(&value, indicator_type);

                let result = if let Some(ref threat_info) = is_threat {
                    format!("THREAT DETECTED: {} - {}", value, threat_info)
                } else {
                    format!("No threat detected for: {}", value)
                };

                println!("{}", result);

                Ok(CommandResult {
                    success: true,
                    message: "Threat check completed".to_string(),
                    data: Some(serde_json::json!({
                        "value": value,
                        "type": r#type,
                        "is_threat": is_threat.is_some(),
                        "threat_info": is_threat
                    })),
                })
            }
        }
    }

    fn execute_info(&self) -> Result<CommandResult> {
        let info = serde_json::json!({
            "name": env!("CARGO_PKG_NAME"),
            "version": env!("CARGO_PKG_VERSION"),
            "description": env!("CARGO_PKG_DESCRIPTION"),
            "authors": env!("CARGO_PKG_AUTHORS"),
            "repository": env!("CARGO_PKG_REPOSITORY"),
            "rust_version": "1.70+",
            "features": ["jsonld", "cyber-security", "rest-api", "cli"]
        });

        let result = serde_json::to_string_pretty(&info)?;
        println!("{}", result);

        Ok(CommandResult {
            success: true,
            message: "System information".to_string(),
            data: Some(info),
        })
    }
}

impl Default for CommandExecutor {
    fn default() -> Self {
        Self::new()
    }
}
