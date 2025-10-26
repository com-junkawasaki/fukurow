//! Reasoner CLI main entry point

use clap::Parser;
use fukurow_cli::{commands::{Cli, CommandExecutor}, interactive::start_interactive};
use tracing_subscriber;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let cli = Cli::parse();

    // Check if running in interactive mode
    if std::env::args().len() == 1 {
        // No arguments provided, start interactive mode
        start_interactive().await?;
        return Ok(());
    }

    // Execute the command
    let mut executor = CommandExecutor::new();
    let result = executor.execute(cli.command).await?;

    // Exit with appropriate code
    if result.success {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}
