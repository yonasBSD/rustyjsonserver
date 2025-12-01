mod commands;

use clap::{Parser, Subcommand};
use commands::{build, serve};
use tracing::error;
use std::error::Error;
use tracing_subscriber::{fmt, EnvFilter};

#[derive(Parser)]
#[command(name = "rjserver")]
#[command(author, version, about = "JSON-driven mock HTTP server")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

impl Cli {
    async fn run(self) -> Result<(), Box<dyn Error>> {
        match self.command {
            Commands::Build(args) => build::run(args).await,
            Commands::Serve(args) => serve::run(args).await,
        }
    }
}

#[derive(Subcommand)]
enum Commands {
    /// Pre-process a JSON config into a standalone file
    Build(commands::build::BuildArgs),

    /// Run the HTTP server
    Serve(commands::serve::ServeArgs),
}

#[tokio::main]
async fn main() {
    // Initialize tracing subscriber with env filter (e.g. RJSERVER_LOG=debug)
    let filter = match EnvFilter::try_from_env("RJSERVER_LOG") {
        Ok(f) => f,
        Err(_) => {
            EnvFilter::new("info")
        }
    };

    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    let cli = Cli::parse();
    if let Err(e) = cli.run().await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }
}
