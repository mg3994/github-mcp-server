use clap::Parser;
use tracing::info;

mod config;
mod error;
mod github;
mod mcp;
mod auth;
mod models;
mod logging;

use config::ServerConfig;
use error::GitHubMcpError;

#[derive(Parser)]
#[command(name = "github-mcp-server")]
#[command(about = "A Model Context Protocol server for GitHub integration")]
struct Args {
    /// Configuration file path
    #[arg(short, long)]
    config: Option<String>,
    
    /// Log level (trace, debug, info, warn, error)
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<(), GitHubMcpError> {
    let args = Args::parse();
    
    // Load configuration first (needed for logging setup)
    let mut config = ServerConfig::from_env()?;
    
    // Override log level from command line if provided
    if args.log_level != "info" {
        config.log_level = args.log_level;
    }
    
    // Initialize logging with configuration
    logging::init_logging(&config)?;
    
    // TODO: Initialize components and start server
    info!("Server initialization complete");
    
    Ok(())
}