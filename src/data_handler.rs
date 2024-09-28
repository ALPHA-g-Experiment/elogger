use crate::config::DataHandlerConfig;
use anyhow::{bail, ensure, Context, Result};
use reqwest::blocking::Response;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use tempfile::Builder;
use tungstenite::client::connect;

#[derive(Serialize)]
struct ClientMessage {
    service: String,
    context: String,
    request: ClientRequest,
}

#[derive(Serialize)]
enum ClientRequest {
    ChronoboxPlot {
        run_number: u32,
        args: ChronoboxTimestampsArgs,
    },
    FinalOdb {
        run_number: u32,
    },
    SequencerCsv {
        run_number: u32,
    },
    SpillLog {
        run_number: u32,
    },
}

#[derive(Deserialize)]
struct ServerMessage {
    service: String,
    context: String,
    response: ServerResponse,
}

#[derive(Deserialize)]
enum ServerResponse {
    Text(String),
    Error(String),
    DownloadJWT(String),
}

// The ALPHA-g Data Handler doesn't (yet) have a stable public API. To get any
// data/plots out of it, it is important that we just reproduce whatever its
// internal web browser client does:
// Make sure that a GET request to the `/:run_number` endpoint returns a
// successful response before requesting anything via websockets.
// This is VERY IMPORTANT (due to some internal state management). Otherwise, we
// would be forcing the server to cache incorrect data.
fn is_data_handler_ready(run_number: u32, config: &DataHandlerConfig) -> Result<bool> {
    Ok(reqwest::blocking::get(format!(
        "http://{}:{}/{run_number}",
        config.host, config.port
    ))
    .with_context(|| format!("failed GET request to data handler's `/{run_number}` endpoint"))?
    .status()
    .is_success())
}

fn ws_request(request: ClientRequest, config: &DataHandlerConfig) -> Result<Response> {
    let msg = ClientMessage {
        service: String::new(),
        context: String::new(),
        request,
    };
    let msg = tungstenite::Message::Text(serde_json::to_string(&msg)?);

    let (mut ws, _) = connect(format!("ws://{}:{}/ws", config.host, config.port))
        .context("failed to connect to data handler websocket")?;
    ws.send(msg)
        .context("failed to send websocket request to data handler")?;

    let jwt = loop {
        if let tungstenite::Message::Text(msg) =
            ws.read().context("failed to read data handler message")?
        {
            let msg: ServerMessage =
                serde_json::from_str(&msg).context("failed to parse data handler message")?;

            match msg.response {
                ServerResponse::DownloadJWT(jwt) => break jwt,
                ServerResponse::Text(_) => continue,
                ServerResponse::Error(err) => {
                    bail!("data handler internal error: `{err}`");
                }
            }
        }
    };

    let resp = reqwest::blocking::get(format!(
        "http://{}:{}/download/{jwt}",
        config.host, config.port
    ))
    .context("failed GET request to data handler's download endpoint")?;
    ensure!(
        resp.status().is_success(),
        "failed to download from data handler"
    );

    Ok(resp)
}

#[derive(Clone, Debug, Deserialize)]
pub struct Record {
    pub sequencer_name: String,
    pub event_description: String,
    pub start_time: f64,
    pub stop_time: f64,
    // Key is a Chronobox channel name
    #[serde(flatten)]
    pub counts: HashMap<String, u32>,
}

pub struct SpillLog {
    pub records: Vec<Record>,
}

pub fn get_spill_log(run_number: u32, config: &DataHandlerConfig) -> Result<SpillLog> {
    ensure!(
        is_data_handler_ready(run_number, config).context("failed to query data handler state")?,
        "data handler is not ready"
    );

    let resp = ws_request(ClientRequest::SpillLog { run_number }, config)
        .context("failed to request spill log from data handler")?;
    let records = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_reader(resp)
        .deserialize()
        .collect::<Result<Vec<Record>, _>>()
        .context("failed to parse spill log")?;

    Ok(SpillLog { records })
}

#[derive(Debug, Deserialize)]
struct SequencerRecord {
    serial_number: u32,
    midas_timestamp: u32,
    header: String,
    xml: String,
}

pub fn get_sequencer_headers(run_number: u32, config: &DataHandlerConfig) -> Result<PathBuf> {
    ensure!(
        is_data_handler_ready(run_number, config).context("failed to query data handler state")?,
        "data handler is not ready"
    );

    let resp = ws_request(ClientRequest::SequencerCsv { run_number }, config)
        .context("failed to request sequencer CSV from data handler")?;
    let records = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_reader(resp)
        .deserialize::<SequencerRecord>()
        .map(|record| record.map(|record| record.header))
        .collect::<Result<Vec<String>, _>>()
        .context("failed to parse sequencer CSV")?
        .join("\n\n\n");

    let mut temp = Builder::new()
        .keep(true)
        .suffix(".txt")
        .tempfile()
        .context("failed to create temporary file")?;
    temp.write_all(records.as_bytes())
        .context("failed to write sequencer headers to temporary file")?;

    Ok(temp.path().to_owned())
}

pub fn get_final_odb(run_number: u32, config: &DataHandlerConfig) -> Result<serde_json::Value> {
    ensure!(
        is_data_handler_ready(run_number, config).context("failed to query data handler state")?,
        "data handler is not ready"
    );

    let text = ws_request(ClientRequest::FinalOdb { run_number }, config)
        .context("failed to request final ODB from data handler")?
        .text()
        .context("failed to read final ODB response text")?;

    let offset = text.find('{').context("failed to find start of ODB JSON")?;
    serde_json::from_str(&text[offset..]).context("failed to parse final ODB")
}

#[derive(Serialize)]
pub struct ChronoboxTimestampsArgs {
    pub board_name: String,
    pub channel_number: u8,
    pub t_bins: Option<u32>,
    pub t_max: Option<f64>,
    pub t_min: Option<f64>,
}

pub fn get_chronobox_plot(
    run_number: u32,
    args: ChronoboxTimestampsArgs,
    config: &DataHandlerConfig,
) -> Result<PathBuf> {
    ensure!(
        is_data_handler_ready(run_number, config).context("failed to query data handler state")?,
        "data handler is not ready"
    );

    let mut temp = Builder::new()
        .keep(true)
        .suffix(".pdf")
        .tempfile()
        .context("failed to create temporary file")?;
    let resp = ws_request(ClientRequest::ChronoboxPlot { run_number, args }, config)
        .context("failed to request chronobox plot from data handler")?
        .copy_to(&mut temp)
        .context("failed to write chronobox plot to temporary file")?;

    Ok(temp.path().to_owned())
}
