use crate::data_handler::{get_spill_log, Record, SpillLog};
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

    let spill_log = get_spill_log(args.run_number, &config.data_handler)
        .context("failed to get spill log from the data handler")?;
    let records = loggable_records(&spill_log, &config.diagnostics);

    Ok(())
}

fn find_diagnostic<'a, T>(record: &Record, diagnostics: &'a T) -> Option<&'a Diagnostic>
where
    &'a T: IntoIterator<Item = &'a Diagnostic>,
{
    diagnostics.into_iter().find(|diagnostic| {
        diagnostic.sequencer_name == record.sequencer_name
            && diagnostic.event_description == record.event_description
    })
}

fn loggable_records<'a, T>(spill_log: &SpillLog, diagnostics: &'a T) -> Vec<Record>
where
    &'a T: IntoIterator<Item = &'a Diagnostic>,
{
    let mut records = spill_log.records.clone();
    records.retain(|record| find_diagnostic(record, diagnostics).is_some());

    records
}
