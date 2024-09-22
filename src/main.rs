use anyhow::Result;
use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(version)]
/// Create an elog for a single run
struct Args {
    /// ALPHA-g run number
    run_number: u32,
    /// Path to a configuration file (overrides the default configuration)
    config_file: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    Ok(())
}
