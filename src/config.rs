use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub spill_log_columns: Vec<String>,
    pub elog: ElogConfig,
    pub data_handler: DataHandlerConfig,
    pub rules: Vec<LogRule>,
}

#[derive(Debug, Deserialize)]
pub struct ElogConfig {
    pub client: PathBuf,
    pub host: String,
    pub port: u16,
    pub logbook: Logbook,
}

#[derive(Debug, Deserialize)]
pub enum Logbook {
    DataLog,
    #[serde(rename = "test")]
    Test,
}

impl std::fmt::Display for Logbook {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Logbook::DataLog => write!(f, "DataLog"),
            Logbook::Test => write!(f, "test"),
        }
    }
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
pub struct ChronoboxTableConfig {
    pub channel_names: Vec<String>,
    #[serde(default)]
    pub include_attachments: bool,
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
