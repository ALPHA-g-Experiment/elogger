use crate::config::{Config, EntryConfig, LogRule};
use crate::data_handler::{get_spill_log, Record, SpillLog};
use anyhow::{Context, Result};
use clap::Parser;
use std::path::PathBuf;

mod config;
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
    let records = {
        let mut records = loggable_records(&spill_log, &config.rules);
        records.sort_by(|a, b| a.record.stop_time.partial_cmp(&b.record.stop_time).unwrap());

        records
    };

    Ok(())
}

#[derive(Debug)]
struct LoggableRecord {
    record: Record,
    config: EntryConfig,
}

fn find_config<'a, T>(record: &Record, rules: &'a T) -> Option<&'a EntryConfig>
where
    &'a T: IntoIterator<Item = &'a LogRule>,
{
    rules
        .into_iter()
        .find(|rule| {
            rule.sequencer_name == record.sequencer_name
                && rule.event_description == record.event_description
        })
        .map(|rule| &rule.config)
}

fn loggable_records<'a, T>(spill_log: &SpillLog, rules: &'a T) -> Vec<LoggableRecord>
where
    &'a T: IntoIterator<Item = &'a LogRule>,
{
    spill_log
        .records
        .iter()
        .filter_map(|record| {
            find_config(record, rules).map(|config| LoggableRecord {
                record: record.clone(),
                config: config.clone(),
            })
        })
        .collect()
}
