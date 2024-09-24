use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    elog: ElogConfig,
    pub data_handler: DataHandlerConfig,
    loggables: Vec<Loggable>,
}

#[derive(Debug, Deserialize)]
struct ElogConfig {
    client: PathBuf,
    host: String,
    port: u16,
    logbook: String,
}

#[derive(Debug, Deserialize)]
pub struct DataHandlerConfig {
    pub host: String,
    pub port: u16,
}

#[derive(Debug, Deserialize)]
struct Loggable {
    sequencer_name: String,
    event_description: String,
    config: RecordConfig,
}

#[derive(Debug, Deserialize)]
struct RecordConfig {
    chronobox_table: Option<ChronoboxTableConfig>,
    #[serde(default)]
    external_resources: Vec<ExternalResourceConfig>,
}

#[derive(Debug, Deserialize)]
struct ChronoboxTableConfig {
    channel_names: Vec<String>,
    #[serde(default)]
    include_attachments: bool,
}

#[derive(Debug, Deserialize)]
struct ExternalResourceConfig {
    base_path: PathBuf,
    header: Option<String>,
    #[serde(default)]
    include_description: bool,
    #[serde(default)]
    include_attachment: bool,
}
