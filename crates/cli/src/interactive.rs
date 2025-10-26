//! Interactive CLI mode

use crate::commands::{CommandExecutor, Cli};
use clap::Parser;
use std::io::{self, Write};
use anyhow::Result;

/// Interactive CLI session
pub struct InteractiveSession {
    executor: CommandExecutor,
}

impl InteractiveSession {
    pub fn new() -> Self {
        Self {
            executor: CommandExecutor::new(),
        }
    }

    /// Start interactive session
    pub async fn run(&mut self) -> Result<()> {
        println!("Welcome to Reasoner CLI Interactive Mode");
        println!("Type 'help' for available commands, 'quit' to exit");
        println!("{}", "=".repeat(50));

        loop {
            print!("reasoner> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let input = input.trim();

            if input.is_empty() {
                continue;
            }

            match input {
                "quit" | "exit" | "q" => {
                    println!("Goodbye!");
                    break;
                }
                "help" | "h" => {
                    self.show_help();
                }
                "clear" => {
                    // Clear screen (Unix-like systems)
                    print!("\x1B[2J\x1B[1;1H");
                }
                _ => {
                    if let Err(e) = self.execute_command(input).await {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }

        Ok(())
    }

    async fn execute_command(&mut self, input: &str) -> Result<()> {
        // Parse the input as CLI arguments
        let args = shell_words::split(input)?;
        let cli = match Cli::try_parse_from(args) {
            Ok(cli) => cli,
            Err(e) => {
                eprintln!("Parse error: {}", e);
                return Ok(());
            }
        };

        let result = self.executor.execute(cli.command).await?;

        if !result.message.is_empty() {
            println!("{}", result.message);
        }

        Ok(())
    }

    fn show_help(&self) {
        println!("Available commands:");
        println!("  serve [options]     Start API server");
        println!("  analyze [options]   Analyze single event");
        println!("  process [options]   Process events from file");
        println!("  query [options]     Query knowledge graph");
        println!("  threat [subcommand] Threat intelligence operations");
        println!("  info                Show system information");
        println!("  help                Show this help");
        println!("  clear               Clear screen");
        println!("  quit                Exit interactive mode");
        println!();
        println!("Use '<command> --help' for detailed help on each command");
    }
}

impl Default for InteractiveSession {
    fn default() -> Self {
        Self::new()
    }
}

/// Start interactive mode
pub async fn start_interactive() -> Result<()> {
    let mut session = InteractiveSession::new();
    session.run().await
}
