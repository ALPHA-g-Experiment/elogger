use crate::config::{Config, Logbook};
use crate::data_handler::{get_final_odb, get_sequencer_headers, get_spill_log};
use crate::summary::spill_log_summary;
use anyhow::{ensure, Context, Result};
use clap::Parser;
use dialoguer::{theme::ColorfulTheme, Input, Select};
use elog::{loggable_records, ElogEntry};
use indicatif::{ProgressBar, ProgressStyle};
use std::ffi::OsStr;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use tempfile::NamedTempFile;

mod config;
mod data_handler;
mod elog;
mod summary;

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

    let parent_id: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Parent message ID (leave empty to create a new thread instead)")
        .allow_empty(true)
        .validate_with(|input: &String| -> Result<(), &str> {
            if input.is_empty() {
                Ok(())
            } else {
                input
                    .parse::<u32>()
                    .map(|_| ())
                    .map_err(|_| "message ID must be a non-negative integer")
            }
        })
        .interact_text()
        .context("failed to read parent message ID")?;

    let mut attributes = Vec::new();
    let author: String = Input::with_theme(&ColorfulTheme::default())
        .with_prompt("Author")
        .interact_text()
        .context("failed to read author")?;
    attributes.push(format!("Author={author}"));
    if let Logbook::DataLog = config.elog.logbook {
        let types = &[
            "Baseline Log",
            "Pbar Log",
            "Trapping Series",
            "Electron Log",
            "Positron Log",
        ];
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Type")
            .default(0)
            .items(&types[..])
            .interact()
            .context("failed to read logbook")?;
        attributes.push(format!("Type={}", types[selection]));
        attributes.push(format!("Run={}", args.run_number));
        attributes.push(format!(
            "Subject={}",
            final_odb
                .pointer("/Experiment/Edit on start/Comment")
                .and_then(serde_json::Value::as_str)
                .map(|s| if s.is_empty() {
                    "MISSING START-RUN COMMENT"
                } else {
                    s
                })
                .context("failed to get comment from ODB")?
        ));
    }

    let spinner = ProgressBar::new_spinner()
        .with_style(ProgressStyle::default_spinner().tick_chars("⠁⠂⠄⡀⢀⠠⠐⠈ "));
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    spinner.set_message("Getting spill log...");
    let spill_log = get_spill_log(args.run_number, &config.data_handler)
        .context("failed to get spill log from the data handler")?;

    let records = {
        let mut records = loggable_records(&spill_log, &config.rules);
        records.sort_by(|a, b| a.record.stop_time.partial_cmp(&b.record.stop_time).unwrap());

        records
    };

    let mut elog_entry = ElogEntry::new();

    spinner.set_message("Logging header...");
    if let Ok(path) = get_sequencer_headers(args.run_number, &config.data_handler) {
        elog_entry.attachments.push(path);
        elog_entry.text.push_str("Sequencer: elog:/1\n");
    }
    if let Ok(path) = spill_log_summary(&spill_log, &config.spill_log_columns) {
        elog_entry.attachments.push(path);
        elog_entry.text.push_str(&format!(
            "Spill log summary: elog:/{}\n",
            elog_entry.attachments.len()
        ));
    }
    elog_entry.text.push_str("\n");

    spinner.set_message("Logging records...");
    for loggable in records {
        elog_entry.add_record(args.run_number, &loggable, &final_odb, &config.data_handler);
    }

    let mut temp_text =
        NamedTempFile::new().context("failed to create temporary elog text file")?;
    temp_text
        .write_all(elog_entry.text.as_bytes())
        .context("failed to write to temporary elog text file")?;

    spinner.set_message("Pushing to server...");
    let mut cmd = Command::new(&config.elog.client);
    cmd.args(["-h", &config.elog.host])
        .args(["-p", &config.elog.port.to_string()])
        .args(["-l", &config.elog.logbook.to_string()])
        .args(
            elog_entry
                .attachments
                .iter()
                .flat_map(|path| [OsStr::new("-f"), path.as_ref()]),
        )
        .args(attributes.iter().flat_map(|attribute| ["-a", attribute]))
        .arg("-x")
        .args(["-n", "1"])
        .args([OsStr::new("-m"), temp_text.path().as_ref()]);
    if !parent_id.is_empty() {
        cmd.args(["-r", &parent_id]);
    }

    let output = cmd.output().context("failed to run the elog client")?;
    ensure!(
        output.status.success(),
        "elog client failed with {}",
        output.status
    );
    spinner.finish_and_clear();
    // The elog client doesn't report errors correctly. With some failure modes,
    // it will still return a successful exit code but print an error message to
    // stdout or stderr. Basically, there is no way to know if the elog was
    // successfully created other than reading all output of the command.
    let _ = std::io::stdout().write_all(&output.stdout);
    let _ = std::io::stderr().write_all(&output.stderr);

    Ok(())
}
