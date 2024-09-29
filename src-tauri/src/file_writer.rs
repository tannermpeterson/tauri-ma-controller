use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{anyhow, bail, Context, Result};
use arrow_array::{ArrayRef, RecordBatch, UInt16Array, UInt64Array};
use arrow_schema::{DataType, SchemaRef};
use arrow_schema::{Field, Schema};
use chrono::{DateTime, Utc};
use parquet::arrow::AsyncArrowWriter;
use serde::Deserialize;
use tokio::{fs::File, sync::mpsc::Receiver};
use tokio_util::sync::CancellationToken;

use crate::packets::{IncomingMagnetometerDataPayload, WellData};
use crate::packets::{SensorData, NUM_WELLS};
use crate::plate::Plate;

// this is not a command because this task is initted with this info
pub struct StartRecordingMetadata {
    plate_barcode: Option<String>,
    stim_barcode: Option<String>,
    is_calibration_recording: bool,
    // TODO add other metadata
}

#[derive(Clone, Debug)]
pub enum FileWriterCommand {
    StopRecording(StopRecordingInfo),
    // TODO what else?
}

#[derive(Clone, Debug, Deserialize)]
pub struct StopRecordingInfo {}

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
    mut rx_ui_to_fw: Receiver<FileWriterCommand>,
    mut rx_ic_to_fw: Receiver<IncomingMagnetometerDataPayload>,
    cancellation_token: CancellationToken,
    start_recording_metadata: StartRecordingMetadata,
) -> Result<()> {
    let mut mag_data_writer = create_mag_data_file(start_recording_metadata).await?;
    let mut mag_data_buffer = MagDataBuffer::new();

    let mut latest_timepoint: u64 = 0;

    let mut cancelled = false;
    // TODO should this loop be in a spawned task?
    loop {
        tokio::select! {
            data = rx_ic_to_fw.recv() => {
                match data {
                    None => {
                        cancelled = true; // assume that if the sender was closed then this task will get cancelled soon
                        break;
                    },
                    Some(data) => {
                        latest_timepoint = data.time_index;
                        mag_data_buffer.push(data);
                        // TODO eventually need to hold on to multiple buffers for a little bit before writing (should only need 2-3?)
                        if mag_data_buffer.full() {
                            let record_batch = RecordBatch::try_from(mag_data_buffer)?;
                            write_and_flush(&mut mag_data_writer, record_batch).await?;
                            mag_data_buffer = MagDataBuffer::new();
                        }
                    },
                }
            }
            command = rx_ui_to_fw.recv() => {
                match command {
                    None => {
                        cancelled = true; // assume that if the sender was closed then this task will get cancelled soon
                        break;
                    },
                    Some(command) => {
                        match command {
                            FileWriterCommand::StopRecording(_) => {
                                break;
                                // TODO what else?
                            }
                        }
                    },
                }
            }
            _ = cancellation_token.cancelled() => {
                cancelled = true;
                break;
            }
        }
    }

    if cancelled {
        // TODO need to handle unwritten data
    } else {
        // TODO
    }

    // TODO any extra handling needed if there is an error here?
    mag_data_writer
        .close()
        .await
        .with_context(|| "Failed to close mag data writer")?;

    // TODO return anything here?
    Ok(())
}

const BUFFER_SIZE: usize = 100;

// TODO try to write a macro that does this?
struct MagDataBuffer {
    pub time_index: Vec<u64>,
    pub well_data: [WellDataBuffer; NUM_WELLS],
}

impl MagDataBuffer {
    fn new() -> Self {
        Self {
            time_index: Vec::with_capacity(BUFFER_SIZE),
            well_data: core::array::from_fn(|_| WellDataBuffer::new()),
        }
    }

    fn push(&mut self, mag_data: IncomingMagnetometerDataPayload) {
        self.time_index.push(mag_data.time_index);
        for well_idx in 0..NUM_WELLS {
            self.well_data[well_idx].push(&mag_data.well_data[well_idx]);
        }
    }

    fn full(&self) -> bool {
        self.time_index.len() == BUFFER_SIZE
    }
}

impl TryFrom<MagDataBuffer> for RecordBatch {
    type Error = anyhow::Error;
    fn try_from(mag_data_buffer: MagDataBuffer) -> Result<Self> {
        let mut cols = vec![(
            "time".to_string(),
            Arc::new(UInt64Array::from(mag_data_buffer.time_index)) as ArrayRef,
        )];

        let plate = Plate::new_24();

        let _ = mag_data_buffer
            .well_data
            .into_iter()
            .enumerate()
            .map(|(i, data)| {
                let well_name = plate.idx_to_name(i, false).expect("should be a valid idx");
                let well_cols = data.into_arrow_cols(&well_name);
                cols.extend(well_cols);
            })
            .collect::<Vec<_>>();

        RecordBatch::try_from_iter(cols)
            .with_context(|| "Failed to convert mag data buf to record batch")
    }
}

pub struct WellDataBuffer {
    pub s1: SensorDataBuffer,
    pub s2: SensorDataBuffer,
    pub s3: SensorDataBuffer,
}

impl WellDataBuffer {
    fn new() -> Self {
        Self {
            s1: SensorDataBuffer::new(),
            s2: SensorDataBuffer::new(),
            s3: SensorDataBuffer::new(),
        }
    }

    fn push(&mut self, well_data: &WellData) {
        self.s1.push(&well_data.s1);
        self.s2.push(&well_data.s2);
        self.s3.push(&well_data.s3);
    }

    fn into_arrow_cols(self, well_name: &str) -> Vec<(String, ArrayRef)> {
        self.s1
            .into_arrow_cols(well_name, 1)
            .into_iter()
            .chain(self.s2.into_arrow_cols(well_name, 2).into_iter())
            .chain(self.s3.into_arrow_cols(well_name, 3).into_iter())
            .collect()
    }
}

pub struct SensorDataBuffer {
    pub time_offset: Vec<u16>,
    pub x: Vec<u16>,
    pub y: Vec<u16>,
    pub z: Vec<u16>,
}

impl SensorDataBuffer {
    fn new() -> Self {
        Self {
            time_offset: Vec::with_capacity(BUFFER_SIZE),
            x: Vec::with_capacity(BUFFER_SIZE),
            y: Vec::with_capacity(BUFFER_SIZE),
            z: Vec::with_capacity(BUFFER_SIZE),
        }
    }

    fn push(&mut self, sensor_data: &SensorData) {
        self.time_offset.push(sensor_data.time_offset);
        self.x.push(sensor_data.x);
        self.y.push(sensor_data.y);
        self.z.push(sensor_data.z);
    }

    fn into_arrow_cols(self, well_name: &str, sensor_num: usize) -> Vec<(String, ArrayRef)> {
        vec![
            (
                format!("{}_{}O", well_name, sensor_num),
                Arc::new(UInt16Array::from(self.time_offset)),
            ),
            (
                format!("{}_{}X", well_name, sensor_num),
                Arc::new(UInt16Array::from(self.x)),
            ),
            (
                format!("{}_{}Y", well_name, sensor_num),
                Arc::new(UInt16Array::from(self.y)),
            ),
            (
                format!("{}_{}Z", well_name, sensor_num),
                Arc::new(UInt16Array::from(self.z)),
            ),
        ]
    }
}

async fn create_mag_data_file(
    start_recording_metadata: StartRecordingMetadata,
) -> Result<AsyncArrowWriter<File>> {
    let utc_beginning_recording: DateTime<Utc> = Utc::now(); // TODO create this based on the metadata
    let file = File::create(format!(
        "default__{}.parquet",
        utc_beginning_recording.format("%Y_%m_%d_%H%M%S")
    ))
    .await?; // TODO make this return a distinct error that can be matched on

    let schema_ref = create_schema_ref().with_context(|| "Failed to create schema ref")?;
    let writer = AsyncArrowWriter::try_new(file, schema_ref, None)?; // TODO make this returns a distinct error that can be matched on
    Ok(writer)
}

fn create_schema_ref() -> Result<SchemaRef> {
    let schema_ref = RecordBatch::try_from(MagDataBuffer::new())?.schema();
    let schema = Arc::into_inner(schema_ref).with_context(|| "Failed getting inner schema")?;
    let schema = schema.with_metadata(create_metadata());
    Ok(Arc::new(schema))
}

fn create_metadata() -> HashMap<String, String> {
    // TODO add real metadata
    let mut metadata = HashMap::new();
    metadata.insert("A".to_string(), "1".to_string());
    metadata.insert("B".to_string(), "2".to_string());
    metadata
}

async fn write_and_flush(
    mag_data_writer: &mut AsyncArrowWriter<File>,
    record_batch: RecordBatch,
) -> Result<()> {
    println!("Writing data");
    let _ = mag_data_writer
        .write(&record_batch)
        .await
        .with_context(|| "Failed to write mag data record batch")?;
    mag_data_writer
        .flush()
        .await
        .with_context(|| "Failed to flush mag data writer")
}
