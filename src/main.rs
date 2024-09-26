use crate::config::Config;
use crate::data_handler::{get_final_odb, get_spill_log};
use anyhow::{Context, Result};
use clap::Parser;
use elog::{loggable_records, ElogEntry};
use std::path::PathBuf;

mod config;
mod data_handler;
mod elog;

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

    let final_odb = get_final_odb(args.run_number, &config.data_handler)
        .context("failed to get the final ODB from the data handler")?;
    let spill_log = get_spill_log(args.run_number, &config.data_handler)
        .context("failed to get spill log from the data handler")?;

    let records = {
        let mut records = loggable_records(&spill_log, &config.rules);
        records.sort_by(|a, b| a.record.stop_time.partial_cmp(&b.record.stop_time).unwrap());

        records
    };

    let mut elog_entry = ElogEntry::new();
    for loggable in records {
        elog_entry.add_record(args.run_number, &loggable, &final_odb, &config.data_handler);
    }

    println!("{}", elog_entry.text);
    println!("{:?}", elog_entry.attachments);

    Ok(())
}
