use anyhow::{Context, Result};
use jiff::{tz::TimeZone, Timestamp, Zoned};
use serde_json::Value;
use std::path::PathBuf;
use std::sync::LazyLock;

// Return the (start_time, stop_time) of a run given the final JSON ODB.
pub fn run_time_limits(odb: &Value) -> Result<(Zoned, Zoned)> {
    let tz = TimeZone::get("Europe/Zurich").context("failed to get `Europe/Zurich` timezone")?;

    let start_time = odb
        .pointer("/Runinfo/Start time binary")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .context("failed to get binary start time")
        .and_then(|s| i64::from_str_radix(s, 16).map_err(|e| anyhow::anyhow!(e)))
        .context("failed to parse start time as i64")?;
    let start_time =
        Timestamp::from_second(start_time).context("failed to create start timestamp")?;
    let start_time = Zoned::new(start_time, tz.clone());

    let stop_time = odb
        .pointer("/Runinfo/Stop time binary")
        .and_then(Value::as_str)
        .and_then(|s| s.strip_prefix("0x"))
        .context("failed to get binary stop time")
        .and_then(|s| i64::from_str_radix(s, 16).map_err(|e| anyhow::anyhow!(e)))
        .context("failed to parse stop time as i64")?;
    let stop_time = Timestamp::from_second(stop_time).context("failed to create stop timestamp")?;
    let stop_time = Zoned::new(stop_time, tz);

    Ok((start_time, stop_time))
}

static FILE_PATTERN: LazyLock<regex::Regex> =
    LazyLock::new(|| regex::Regex::new(r"^\d{2}\d{2}_\d{2}\.\d{3}\.png$").unwrap());
// An external resource is created with the name
// `base_path/<year>/<month>/<day>/hhmm_ss.mss.png`
// Return all paths in chronological order.
pub fn find_external_resources(
    base_path: PathBuf,
    start_time: Zoned,
    stop_time: Zoned,
) -> Result<Vec<PathBuf>> {
    let first_possible_entry = base_path
        .join(start_time.year().to_string())
        .join(format!("{:02}", start_time.month()))
        .join(format!("{:02}", start_time.day()))
        .join(format!(
            "{:02}{:02}_{:02}.000.png",
            start_time.hour(),
            start_time.minute(),
            start_time.second(),
        ));
    let last_possible_entry = base_path
        .join(stop_time.year().to_string())
        .join(format!("{:02}", stop_time.month()))
        .join(format!("{:02}", stop_time.day()))
        .join(format!(
            "{:02}{:02}_{:02}.999.png",
            stop_time.hour(),
            stop_time.minute(),
            stop_time.second(),
        ));

    let mut paths = Vec::new();

    let mut date = start_time.date();
    while date <= stop_time.date() {
        let folder = base_path
            .join(date.year().to_string())
            .join(format!("{:02}", date.month()))
            .join(format!("{:02}", date.day()));

        let mut entries = std::fs::read_dir(&folder)
            .context(format!("failed to read directory `{}`", folder.display()))?
            .filter_map(|res| res.map(|e| e.path()).ok())
            .filter(|p| {
                p.is_file()
                    && p.file_name()
                        .and_then(|f| f.to_str())
                        .map(|f| FILE_PATTERN.is_match(f))
                        .unwrap_or(false)
            })
            .collect::<Vec<_>>();
        if date == start_time.date() {
            entries.retain(|p| p > &first_possible_entry);
        }
        if date == stop_time.date() {
            entries.retain(|p| p < &last_possible_entry);
        }
        entries.sort_unstable();

        paths.extend(entries);

        date = date.tomorrow().unwrap();
    }

    Ok(paths)
}
