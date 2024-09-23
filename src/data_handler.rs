use crate::DataHandler;
use anyhow::{bail, ensure, Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use tungstenite::client::connect;

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
fn is_data_handler_ready(run_number: u32, config: &DataHandler) -> Result<bool> {
    Ok(reqwest::blocking::get(format!(
        "http://{}:{}/{run_number}",
        config.host, config.port
    ))
    .with_context(|| format!("failed GET request to data handler's `/{run_number}` endpoint"))?
    .status()
    .is_success())
}

#[derive(Deserialize)]
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

pub fn get_spill_log(run_number: u32, config: &DataHandler) -> Result<SpillLog> {
    ensure!(
        is_data_handler_ready(run_number, config).context("failed to query data handler state")?,
        "data handler is not ready"
    );

    let (mut ws, _) = connect(format!("ws://{}:{}/ws", config.host, config.port))
        .context("failed to connect to data handler websocket")?;
    let message = tungstenite::Message::Text(format!(
        r#"{{"service": "", "context": "", "request": {{"SpillLog": {{"run_number": {run_number}}}}}}}"#
    ));
    ws.send(message)
        .context("failed to send spill log request to data handler")?;

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
    ensure!(resp.status().is_success(), "failed to download spill log");

    let records = csv::ReaderBuilder::new()
        .comment(Some(b'#'))
        .from_reader(resp)
        .deserialize()
        .collect::<Result<Vec<Record>, _>>()
        .context("failed to parse spill log")?;
    Ok(SpillLog { records })
}
