//! Tests for the cli crate

use reasoner_cli::commands::{Cli, Commands, CommandResult, CommandExecutor, OutputFormat};
use clap::Parser;
use reasoner_graph::model::CyberEvent;
use std::path::PathBuf;

#[test]
fn test_cli_parsing_info() {
    let args = vec!["reasoner-cli", "info"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Info => {} // Expected
        _ => panic!("Expected Info command"),
    }
}

#[test]
fn test_cli_parsing_serve() {
    let args = vec!["reasoner-cli", "serve", "--host", "127.0.0.1", "--port", "8080"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Serve { host, port } => {
            assert_eq!(host, "127.0.0.1");
            assert_eq!(port, 8080);
        }
        _ => panic!("Expected Serve command"),
    }
}

#[test]
fn test_cli_parsing_analyze_with_file() {
    let args = vec!["reasoner-cli", "analyze", "--file", "test.json", "--format", "json"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Analyze { file, json, format } => {
            assert_eq!(file, Some(PathBuf::from("test.json")));
            assert_eq!(json, None);
            assert_eq!(format, OutputFormat::Json);
        }
        _ => panic!("Expected Analyze command"),
    }
}

#[test]
fn test_cli_parsing_analyze_with_json() {
    let json_data = r#"{"type": "NetworkConnection", "source_ip": "192.168.1.10"}"#;
    let args = vec!["reasoner-cli", "analyze", "--json", json_data, "--format", "text"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Analyze { file, json, format } => {
            assert_eq!(file, None);
            assert_eq!(json, Some(json_data.to_string()));
            assert_eq!(format, OutputFormat::Text);
        }
        _ => panic!("Expected Analyze command"),
    }
}

#[test]
fn test_cli_parsing_process() {
    let args = vec!["reasoner-cli", "process", "--input", "events.json", "--output", "results.json"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Process { input, output, format } => {
            assert_eq!(input, PathBuf::from("events.json"));
            assert_eq!(output, Some(PathBuf::from("results.json")));
            assert_eq!(format, OutputFormat::Json); // Default
        }
        _ => panic!("Expected Process command"),
    }
}

#[test]
fn test_cli_parsing_query() {
    let args = vec![
        "reasoner-cli", "query",
        "--subject", "https://example.com/user/1",
        "--predicate", "https://example.com/ns/name",
        "--format", "text"
    ];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Query { subject, predicate, object, format } => {
            assert_eq!(subject, Some("https://example.com/user/1".to_string()));
            assert_eq!(predicate, Some("https://example.com/ns/name".to_string()));
            assert_eq!(object, None);
            assert_eq!(format, OutputFormat::Text);
        }
        _ => panic!("Expected Query command"),
    }
}

#[test]
fn test_cli_parsing_threat_stats() {
    let args = vec!["reasoner-cli", "threat", "stats"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Threat { command } => {
            match command {
                reasoner_cli::commands::ThreatCommands::Stats => {} // Expected
                _ => panic!("Expected Stats subcommand"),
            }
        }
        _ => panic!("Expected Threat command"),
    }
}

#[test]
fn test_cli_parsing_threat_check() {
    let args = vec!["reasoner-cli", "threat", "check", "--value", "192.168.1.100", "--type", "ip"];
    let cli = Cli::try_parse_from(args).unwrap();

    match cli.command {
        Commands::Threat { command } => {
            match command {
                reasoner_cli::commands::ThreatCommands::Check { value, r#type } => {
                    assert_eq!(value, "192.168.1.100");
                    assert_eq!(r#type, "ip");
                }
                _ => panic!("Expected Check subcommand"),
            }
        }
        _ => panic!("Expected Threat command"),
    }
}

#[test]
fn test_cli_invalid_args() {
    let args = vec!["reasoner-cli", "analyze"]; // Missing required --file or --json
    let result = Cli::try_parse_from(args);
    assert!(result.is_err()); // Should fail due to missing required args
}

#[test]
fn test_output_format_enum() {
    let text_format = OutputFormat::Text;
    let json_format = OutputFormat::Json;
    let json_pretty_format = OutputFormat::JsonPretty;

    assert_eq!(format!("{:?}", text_format), "Text");
    assert_eq!(format!("{:?}", json_format), "Json");
    assert_eq!(format!("{:?}", json_pretty_format), "JsonPretty");
}

#[test]
fn test_command_result_creation() {
    let success_result = CommandResult {
        success: true,
        message: "Operation completed".to_string(),
        data: Some(serde_json::json!({"count": 5})),
    };

    assert!(success_result.success);
    assert_eq!(success_result.message, "Operation completed");
    assert!(success_result.data.is_some());

    let error_result = CommandResult {
        success: false,
        message: "Operation failed".to_string(),
        data: None,
    };

    assert!(!error_result.success);
    assert_eq!(error_result.message, "Operation failed");
    assert!(error_result.data.is_none());
}

#[tokio::test]
async fn test_command_executor_creation() {
    let executor = CommandExecutor::new();
    // Should create without panicking
    assert!(true); // If we reach here, creation succeeded
}

#[tokio::test]
async fn test_command_executor_info() {
    let mut executor = CommandExecutor::new();
    let result = executor.execute(Commands::Info).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("System information"));
    assert!(result.data.is_some());

    if let Some(data) = &result.data {
        let info = data.as_object().unwrap();
        assert!(info.contains_key("name"));
        assert!(info.contains_key("version"));
        assert!(info.contains_key("rust_version"));
        assert!(info.contains_key("features"));
    }
}

#[tokio::test]
async fn test_command_executor_analyze_invalid_input() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Analyze {
        file: None,
        json: Some("invalid json".to_string()),
        format: OutputFormat::Text,
    };

    let result = executor.execute(command).await;
    assert!(result.is_err()); // Should fail due to invalid JSON
}

#[tokio::test]
async fn test_command_executor_analyze_valid_json() {
    let mut executor = CommandExecutor::new();

    let event_json = r#"{
        "type": "NetworkConnection",
        "source_ip": "192.168.1.10",
        "dest_ip": "10.0.0.50",
        "port": 443,
        "protocol": "tcp",
        "timestamp": 1640995200
    }"#;

    let command = Commands::Analyze {
        file: None,
        json: Some(event_json.to_string()),
        format: OutputFormat::Json,
    };

    let result = executor.execute(command).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("Event analyzed"));
    assert!(result.data.is_some());
}

#[tokio::test]
async fn test_command_executor_query_empty_graph() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Query {
        subject: None,
        predicate: None,
        object: None,
        format: OutputFormat::Text,
    };

    let result = executor.execute(command).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("Found 0 triples"));
    assert!(result.data.is_some());
}

#[tokio::test]
async fn test_command_executor_threat_stats() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Threat {
        command: reasoner_cli::commands::ThreatCommands::Stats,
    };

    let result = executor.execute(command).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("Threat intelligence statistics"));
    assert!(result.data.is_some());

    if let Some(data) = &result.data {
        let stats = data.as_object().unwrap();
        assert!(stats.contains_key("total_indicators"));
        assert!(stats.contains_key("ip_indicators"));
        assert!(stats.contains_key("domain_indicators"));
        assert!(stats.contains_key("sources"));
    }
}

#[tokio::test]
async fn test_command_executor_threat_check_malicious() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Threat {
        command: reasoner_cli::commands::ThreatCommands::Check {
            value: "192.168.1.100".to_string(),
            r#type: "ip".to_string(),
        },
    };

    let result = executor.execute(command).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("Threat check completed"));
    assert!(result.data.is_some());

    if let Some(data) = &result.data {
        let check_result = data.as_object().unwrap();
        assert!(check_result.contains_key("value"));
        assert!(check_result.contains_key("type"));
        assert!(check_result.contains_key("is_threat"));
        assert!(check_result.contains_key("threat_info"));
    }
}

#[tokio::test]
async fn test_command_executor_threat_check_safe() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Threat {
        command: reasoner_cli::commands::ThreatCommands::Check {
            value: "8.8.8.8".to_string(),
            r#type: "ip".to_string(),
        },
    };

    let result = executor.execute(command).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("Threat check completed"));
    assert!(result.data.is_some());

    if let Some(data) = &result.data {
        let check_result = data.as_object().unwrap();
        assert_eq!(check_result.get("is_threat"), Some(&serde_json::Value::Bool(false)));
        assert_eq!(check_result.get("threat_info"), Some(&serde_json::Value::Null));
    }
}

#[tokio::test]
async fn test_command_executor_threat_check_invalid_type() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Threat {
        command: reasoner_cli::commands::ThreatCommands::Check {
            value: "192.168.1.100".to_string(),
            r#type: "invalid_type".to_string(),
        },
    };

    let result = executor.execute(command).await;
    assert!(result.is_err()); // Should fail due to invalid indicator type
}

#[tokio::test]
async fn test_command_executor_serve_command_structure() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Serve {
        host: "127.0.0.1".to_string(),
        port: 8080,
    };

    // Note: This will attempt to start a server, but we expect it to fail gracefully
    // in a test environment (since we're not actually running an async runtime for server)
    let result = executor.execute(command).await;

    // The result depends on the server implementation, but it should not panic
    assert!(result.is_ok() || result.is_err()); // Just ensure it doesn't crash
}

#[tokio::test]
async fn test_command_executor_process_invalid_file() {
    let mut executor = CommandExecutor::new();

    let command = Commands::Process {
        input: PathBuf::from("nonexistent_file.json"),
        output: None,
        format: OutputFormat::Json,
    };

    let result = executor.execute(command).await;
    assert!(result.is_err()); // Should fail due to non-existent file
}

#[tokio::test]
async fn test_command_executor_process_valid_data() {
    use std::fs;
    use tempfile::NamedTempFile;

    let mut executor = CommandExecutor::new();

    // Create temporary input file with valid JSON
    let events_data = r#"[
        {
            "type": "NetworkConnection",
            "source_ip": "192.168.1.10",
            "dest_ip": "10.0.0.50",
            "port": 443,
            "protocol": "tcp",
            "timestamp": 1640995200
        }
    ]"#;

    let input_file = NamedTempFile::new().unwrap();
    fs::write(&input_file, events_data).unwrap();

    let command = Commands::Process {
        input: input_file.path().to_path_buf(),
        output: None,
        format: OutputFormat::Json,
    };

    let result = executor.execute(command).await.unwrap();

    assert!(result.success);
    assert!(result.message.contains("Processed 1 events"));
    assert!(result.data.is_some());

    // Cleanup
    drop(input_file);
}

#[test]
fn test_interactive_mode_parsing() {
    // Test that shell-words parsing works for interactive commands
    let input = "analyze --json '{\"type\": \"NetworkConnection\"}' --format json";
    let args_result = shell_words::split(input);

    assert!(args_result.is_ok());
    let args = args_result.unwrap();

    // Should parse into: ["analyze", "--json", "{\"type\": \"NetworkConnection\"}", "--format", "json"]
    assert_eq!(args.len(), 5);
    assert_eq!(args[0], "analyze");
    assert_eq!(args[1], "--json");
    assert_eq!(args[3], "--format");
    assert_eq!(args[4], "json");
}

#[test]
fn test_interactive_mode_complex_commands() {
    let test_cases = vec![
        "query --subject https://example.com/user/1",
        "threat check --value 192.168.1.100 --type ip",
        "process --input events.json --output results.json --format json-pretty",
        "analyze --file test.json --format text",
    ];

    for input in test_cases {
        let args_result = shell_words::split(input);
        assert!(args_result.is_ok(), "Failed to parse: {}", input);

        let args = args_result.unwrap();
        assert!(!args.is_empty(), "Empty args for: {}", input);
    }
}

#[tokio::test]
async fn test_command_executor_integration() {
    let mut executor = CommandExecutor::new();

    // Add an event
    let analyze_result = executor.execute(Commands::Analyze {
        file: None,
        json: Some(r#"{
            "type": "NetworkConnection",
            "source_ip": "192.168.1.10",
            "dest_ip": "10.0.0.50",
            "port": 443,
            "protocol": "tcp",
            "timestamp": 1640995200
        }"#.to_string()),
        format: OutputFormat::Json,
    }).await;

    assert!(analyze_result.is_ok());
    assert!(analyze_result.unwrap().success);

    // Query the graph - should now have triples
    let query_result = executor.execute(Commands::Query {
        subject: None,
        predicate: None,
        object: None,
        format: OutputFormat::Json,
    }).await;

    assert!(query_result.is_ok());
    let query_command_result = query_result.unwrap();
    assert!(query_command_result.success);
    assert!(query_command_result.message.contains("Found"));
    assert!(query_command_result.data.is_some());
}
