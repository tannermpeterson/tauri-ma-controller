// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use anyhow::{bail, Context, Result};
use packets::IncomingMagnetometerDataPayload;
use serde::Deserialize;
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex,
};
use tokio_util::sync::CancellationToken;

mod file_writer;
mod instrument_comm;
mod packets;
use file_writer::{run_file_writer, StartRecordingMetadata};
use instrument_comm::{run_instrument_comm, InstrumentCommand};

fn main() {
    tauri::Builder::default()
        .manage(Mutex::new(ControllerState::new()))
        .invoke_handler(tauri::generate_handler![
            connect,
            cancel,
            start_data_stream,
            stop_data_stream,
            record
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

struct ControllerState {
    tx_ic: Option<Sender<InstrumentCommand>>,
    cancellation_token: Option<CancellationToken>,
}

impl ControllerState {
    fn new() -> Self {
        Self {
            tx_ic: None,
            cancellation_token: None,
        }
    }

    fn clear(&mut self) {
        *self = Self::new();
    }
}

#[derive(Deserialize)]
struct StartRecordingCommandInfo {
    plate_barcode: Option<String>,
    stim_barcode: Option<String>,
    is_calibration_recording: bool,
    // TODO platemap_info
}

fn handle_err(e: anyhow::Error) -> String {
    let err_str = format!("{:#?}", e);
    println!("ERROR: {}", err_str);
    err_str
}

#[tauri::command]
async fn connect(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    println!("Received connect cmd");
    connect_internal(state)
        .await
        .map_err(|e| format!("{:#?}", e))
}

#[tauri::command]
async fn record(
    state: tauri::State<'_, Mutex<ControllerState>>,
    command_info: StartRecordingCommandInfo,
) -> Result<(), String> {
    println!("Received record cmd");
    if command_info.is_calibration_recording && command_info.plate_barcode.is_none() {
        let err_msg = "plate_barcode must be set for tissue recordings";
        println!("ERROR: {}", err_msg);
        return Err(err_msg.to_string());
    }

    let (tx_ic_to_fw, rx_ic_to_fw) = channel(10);
    send_command_to_ic(InstrumentCommand::StartRecording(tx_ic_to_fw), &state).await?;
    record_internal(state, rx_ic_to_fw, command_info)
        .await
        .map_err(|e| handle_err(e))
}

#[tauri::command]
async fn start_data_stream(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    send_command_to_ic(InstrumentCommand::StartDataStream, &state).await
}

#[tauri::command]
async fn stop_data_stream(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    send_command_to_ic(InstrumentCommand::StopDataStream, &state).await
}

#[tauri::command]
async fn cancel(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    let mut state = state.lock().await;
    if let Some(ref cancellation_token) = state.cancellation_token {
        println!("Cancelling");
        cancellation_token.cancel();
    }
    state.clear();
    Ok(())
}

async fn connect_internal(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<()> {
    // TODO in future, could spawn other tasks or just create a simple loop here to handle writing
    // data to file, analyzing data, sending back to UI

    let cancellation_token = CancellationToken::new();
    let cancellation_token_ic = cancellation_token.clone();
    let (tx_ic, rx_ic) = channel(10);

    {
        let mut state = state.lock().await;
        state.tx_ic = Some(tx_ic);
        state.cancellation_token = Some(cancellation_token);
    }

    run_instrument_comm(rx_ic, cancellation_token_ic)
        .await
        .with_context(|| "Error in Instrument Comm")
}

async fn record_internal(
    state: tauri::State<'_, Mutex<ControllerState>>,
    rx_ic_to_fw: Receiver<IncomingMagnetometerDataPayload>,
    command_info: StartRecordingCommandInfo,
) -> Result<()> {
    // TODO eventually might need a way to send commands from here to file writer
    let cancellation_token_fw = {
        let state = state.lock().await;
        if let Some(ref cancellation_token) = state.cancellation_token {
            cancellation_token.clone()
        } else {
            bail!("No cancellation token created");
        }
    };

    let start_recording_metadata = StartRecordingMetadata::new(
        command_info.plate_barcode,
        command_info.stim_barcode,
        command_info.is_calibration_recording,
        // TODO add other info
    );

    run_file_writer(rx_ic_to_fw, cancellation_token_fw, start_recording_metadata)
        .await
        .with_context(|| "Error in Instrument Comm")
}

async fn send_command_to_ic(
    command: InstrumentCommand,
    state: &tauri::State<'_, Mutex<ControllerState>>,
) -> Result<(), String> {
    if let Some(ref tx_ic) = state.lock().await.tx_ic {
        tx_ic
            .send(command.clone())
            .await
            .with_context(|| format!("Failed to send command: {:?}", command))
            .map_err(|e| handle_err(e))
    } else {
        Err("Instrument is not connected".into())
    }
}
