use anyhow::{Context, Result};
use clap::Parser;
use serde::Deserialize;
use std::path::PathBuf;

mod data_handler;

#[derive(Parser)]
#[command(version)]
/// Create an elog for a single run
struct Args {
    /// ALPHA-g run number
    run_number: u32,
    /// Path to a configuration file (overrides the default configuration)
    #[arg(short, long)]
    config_file: Option<PathBuf>,
}

#[derive(Deserialize)]
struct Config {
    elog: Elog,
    data_handler: DataHandler,
    diagnostics: Vec<Diagnostic>,
}

#[derive(Deserialize)]
struct Elog {
    client: PathBuf,
    host: String,
    port: u16,
    logbook: String,
}

#[derive(Deserialize)]
struct DataHandler {
    host: String,
    port: u16,
}

#[derive(Deserialize)]
struct Diagnostic {
    sequencer_name: String,
    event_description: String,
    #[serde(default)]
    chronobox_channels: Vec<String>,
    // Each base_path corresponds to a different external timed resource, e.g.
    // MCP images, CsI transients, etc.
    #[serde(default)]
    base_paths: Vec<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let config = args.config_file.unwrap_or_else(|| {
        directories::ProjectDirs::from("com", "ALPHA", "ALPHA-g-Elogger")
            .unwrap()
            .config_local_dir()
            .join("Elogger.toml")
    });
    let config = std::fs::read_to_string(&config)
        .with_context(|| format!("failed to read `{}`", config.display()))?;
    let config: Config = toml::from_str(&config).context("failed to parse configuration")?;

    Ok(())
}
