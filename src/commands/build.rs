use std::{error::Error, fs, io, path::{Path, PathBuf}};
use clap::Args;
use serde_json;
use rustyjsonserver::config::resolver::{get_config_path_cwd, load_config, resolve_config_references};
use tracing::info;

/// Pre-process a JSON config into a standalone file.
#[derive(Args, Debug)]
pub struct BuildArgs {
    /// Input config file
    #[arg(short, long, value_name = "FILE")]
    pub config: PathBuf,

    /// Output filename for the processed JSON
    #[arg(short, long, value_name = "FILE")]
    pub output: PathBuf,
}

pub async fn run(args: BuildArgs) -> Result<(), Box<dyn Error>> {
    let cfg = get_config_path_cwd(&args.config.to_string_lossy());
    let out = get_config_path_cwd(&args.output.to_string_lossy());
    info!(%cfg, %out, "starting build");

    // 1) Load
    let config = load_config(&cfg)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("load_config failed: {}", e)))?;

    // 2) Inline references
    let root = PathBuf::from(&cfg)
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();
    let final_conf = resolve_config_references(config, &root)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("resolve_config_references failed: {}", e)))?;

    // 3) Serialize + write
    let json = serde_json::to_string_pretty(&final_conf)?;
    fs::write(&out, json)?;

    info!("build succeeded");
    Ok(())
}
