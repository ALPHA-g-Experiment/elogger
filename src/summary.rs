use crate::data_handler::SpillLog;
use anyhow::{Context, Result};
use std::io::Write;
use std::path::PathBuf;

pub fn spill_log_summary(spill_log: &SpillLog, columns: &[String]) -> Result<PathBuf> {
    let mut builder = tabled::builder::Builder::new();

    let mut header = vec![
        "Event".to_string(),
        "Start time".to_string(),
        "Stop time".to_string(),
    ];
    header.extend(columns.iter().cloned());
    builder.push_record(header);

    for record in spill_log.records.iter() {
        let mut row = vec![
            format!(
                "{} - {}",
                record.sequencer_name.to_uppercase(),
                record.event_description
            ),
            record.start_time.to_string(),
            record.stop_time.to_string(),
        ];

        for column in columns.iter() {
            row.push(
                record
                    .counts
                    .get(column)
                    .map_or_else(|| String::from("<NOT_IN_SPILL_LOG>"), u32::to_string),
            );
        }

        builder.push_record(row);
    }

    let mut temp = tempfile::Builder::new()
        .keep(true)
        .suffix(".txt")
        .tempfile()
        .context("failed to create temporary file")?;
    temp.write_all(builder.build().to_string().as_bytes())
        .context("failed to write to temporary file")?;

    Ok(temp.path().to_owned())
}
