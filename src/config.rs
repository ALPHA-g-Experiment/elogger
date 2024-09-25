use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    elog: ElogConfig,
    pub data_handler: DataHandlerConfig,
    pub rules: Vec<LogRule>,
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
pub struct LogRule {
    pub sequencer_name: String,
    pub event_description: String,
    pub config: EntryConfig,
}

#[derive(Clone, Debug, Deserialize)]
pub struct EntryConfig {
    pub chronobox_table: Option<ChronoboxTableConfig>,
    #[serde(default)]
    pub external_resources: Vec<ExternalResourceConfig>,
}

#[derive(Clone, Debug, Deserialize)]
struct ChronoboxTableConfig {
    channel_names: Vec<String>,
    #[serde(default)]
    include_attachments: bool,
}

#[derive(Clone, Debug, Deserialize)]
struct ExternalResourceConfig {
    base_path: PathBuf,
    header: Option<String>,
    #[serde(default)]
    include_description: bool,
    #[serde(default)]
    include_attachment: bool,
}
