use crate::config::{DataHandlerConfig, EntryConfig, LogRule};
use crate::data_handler::{get_chronobox_plot, ChronoboxTimestampsArgs, Record, SpillLog};
use anyhow::{ensure, Context, Result};
use std::path::PathBuf;

#[derive(Debug)]
pub struct LoggableRecord {
    pub record: Record,
    pub config: EntryConfig,
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

pub fn loggable_records<'a, T>(spill_log: &SpillLog, rules: &'a T) -> Vec<LoggableRecord>
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

#[derive(Debug)]
struct ChronoboxChannel {
    board_name: String,
    channel_number: u8,
}

fn find_chronobox_channel(channel_name: &str, odb: &serde_json::Value) -> Result<ChronoboxChannel> {
    let mut found_channels = Vec::new();
    for board_name in ["cb01", "cb02", "cb03", "cb04"] {
        let names = &odb["Equipment"][board_name]["Settings"]["names"]
            .as_array()
            .context("failed to find chronobox names array in the ODB")?;
        for (channel_number, name) in names.iter().filter_map(|n| n.as_str()).enumerate() {
            if name == channel_name {
                found_channels.push(ChronoboxChannel {
                    board_name: board_name.to_string(),
                    channel_number: channel_number as u8,
                });
            }
        }
    }

    ensure!(
        found_channels.len() == 1,
        "failed to find a unique channel with name `{channel_name}` in the ODB",
    );
    Ok(found_channels.pop().unwrap())
}

pub struct ElogEntry {
    pub text: String,
    pub attachments: Vec<PathBuf>,
}

impl ElogEntry {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            attachments: Vec::new(),
        }
    }

    pub fn add_record(
        &mut self,
        run_number: u32,
        loggable: &LoggableRecord,
        odb: &serde_json::Value,
        handler_config: &DataHandlerConfig,
    ) {
        let mut new_text = format!(
            "{} - {}\n",
            loggable.record.sequencer_name.to_uppercase(),
            loggable.record.event_description
        );

        if let Some(table_config) = &loggable.config.chronobox_table {
            let mut header = table_config.channel_names.clone();
            let mut data = header
                .iter()
                .map(|channel| {
                    loggable
                        .record
                        .counts
                        .get(channel)
                        .map_or_else(|| String::from("<NOT_IN_SPILL_LOG>"), u32::to_string)
                })
                .collect::<Vec<_>>();

            if table_config.include_attachments {
                for channel in &header {
                    if let Ok(channel) = find_chronobox_channel(channel, odb) {
                        let args = ChronoboxTimestampsArgs {
                            board_name: channel.board_name,
                            channel_number: channel.channel_number,
                            t_bins: None,
                            t_max: Some(loggable.record.stop_time),
                            t_min: Some(loggable.record.start_time),
                        };
                        if let Ok(path) = get_chronobox_plot(run_number, args, handler_config) {
                            self.attachments.push(path);
                            data.push(format!("elog:/{}", self.attachments.len()));
                        } else {
                            data.push(String::from("<DATA_HANDLER_ERROR>"));
                        }
                    } else {
                        data.push(String::from("<NOT_IN_ODB>"));
                    }
                }
                header.extend(std::iter::repeat(String::new()).take(header.len()));
            }

            let mut builder = tabled::builder::Builder::new();
            builder.push_record(header);
            builder.push_record(data);
            new_text.push_str(&format!("{}\n", builder.build()));
        }

        self.text.push_str(&indent::indent_by(4, new_text));
    }
}
