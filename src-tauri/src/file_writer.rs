use anyhow::{anyhow, bail, Context, Result};
use tokio::sync::mpsc::Receiver;
use tokio_util::sync::CancellationToken;

use crate::packets::IncomingMagnetometerDataPayload;

pub struct StartRecordingMetadata {
    plate_barcode: Option<String>,
    stim_barcode: Option<String>,
    is_calibration_recording: bool,
}

impl StartRecordingMetadata {
    pub fn new(
        plate_barcode: Option<String>,
        stim_barcode: Option<String>,
        is_calibration_recording: bool,
    ) -> Self {
        Self {
            plate_barcode,
            stim_barcode,
            is_calibration_recording,
        }
    }
}

pub async fn run_file_writer(
    rx_ic_to_fw: Receiver<IncomingMagnetometerDataPayload>,
    cancellation_token: CancellationToken,
    start_recording_metadata: StartRecordingMetadata,
) -> Result<()> {
    // TODO create the file with the default name
    // TODO set up select loop to wait for a new data packet or the cancellation token
    Ok(()) // TODO
}
