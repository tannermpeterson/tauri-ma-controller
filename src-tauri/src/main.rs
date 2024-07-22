// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{collections::VecDeque, future, io};
use tokio::{
    io::AsyncWriteExt,
    net::{
        tcp::{ReadHalf, WriteHalf},
        TcpStream,
    },
    sync::{
        mpsc::{channel, Receiver, Sender},
        Mutex,
    },
    task::{JoinError, JoinSet, LocalSet},
    time::{sleep_until, timeout, Duration, Instant},
};

mod packets;
use anyhow::{anyhow, bail, Context, Result};
use packets::{
    is_magic_word, parse_data_stream, OutgoingPacket, OutgoingPacketPayloads, PacketTypes,
    ParsedMagnetometerData, MAGIC_WORD_LEN,
};
use tokio_util::sync::CancellationToken;

use crate::packets::IncomingPacketPayloads;

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

#[derive(Debug, Clone)]
enum InstrumentCommand {
    StartDataStream,
    StopDataStream,
    StartRecording(Sender<ParsedMagnetometerData>),
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

fn handle_err(e: anyhow::Error) -> String {
    let err_str = format!("{:#?}", e);
    println!("ERROR: {}", err_str);
    err_str
}

#[tauri::command]
async fn connect(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    connect_internal(state)
        .await
        .map_err(|e| format!("{:#?}", e))
}

#[tauri::command]
async fn record(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    // TODO
    Ok(())
    // let (tx_ic_to_fw, mut rx_ic_to_fw) = channel(10);
    // send_command_to_ic(InstrumentCommand::StartRecording(tx_ic_to_fw), state);
    // record_internal(state, rx_ic_to_fw)
    //     .await
    //     .map_err(|e| handle_err(e))
}

#[tauri::command]
async fn start_data_stream(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    send_command_to_ic(InstrumentCommand::StartDataStream, state).await
}

#[tauri::command]
async fn stop_data_stream(state: tauri::State<'_, Mutex<ControllerState>>) -> Result<(), String> {
    send_command_to_ic(InstrumentCommand::StopDataStream, state).await
}

async fn send_command_to_ic(
    command: InstrumentCommand,
    state: tauri::State<'_, Mutex<ControllerState>>,
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

// TODO double check all the error handling from here to the end of the file
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

// TODO move all IC stuff into another file
async fn run_instrument_comm(
    rx_ui_to_ic: Receiver<InstrumentCommand>,
    cancellation_token: CancellationToken,
) -> Result<()> {
    let (tx_ic_to_iio, rx_ic_to_iio) = channel(10);
    let (tx_iio_to_ic, rx_iio_to_ic) = channel(10);

    let mut js = JoinSet::new();
    js.spawn(async move {
        run_instrument_comm_internal(tx_ic_to_iio, rx_iio_to_ic, rx_ui_to_ic)
            .await
            .with_context(|| "Error in run_instrument_comm_internal")
    });

    js.spawn(async move {
        instrument_io(tx_iio_to_ic, rx_ic_to_iio)
            .await
            .with_context(|| "Error in instrument_io")
    });

    // TODO clean up the names of these tasks
    let mut base_res = Ok(());
    tokio::select! {
        res = js.join_next() => {
            if let Some(res) = res {
                base_res = complete_task(res);
            }
            // TODO cancel token here?
        }
        _ = cancellation_token.cancelled() => {
            js.abort_all();
        }
    }

    while let Some(res) = js.join_next().await {
        complete_task(res);
    }

    base_res
}

fn complete_task(join_res: Result<Result<()>, JoinError>) -> Result<()> {
    let res = match join_res {
        Ok(task_res) => task_res,
        Err(join_err) => {
            if join_err.is_cancelled() {
                Ok(())
            } else if join_err.is_panic() {
                // TODO how to handle this?
                println!("PANIC IN TASK: {}", join_err);
                Ok(())
            } else {
                Err(join_err).with_context(|| "Failed joining task")
            }
        }
    };
    // TODO clean this up, also try to get the name of the task
    if let Err(ref e) = res {
        println!("Error in Instrument Comm: {:#?}", e);
    } else {
        println!("Instrument Comm task joined");
    }
    res
}

const PACKET_SYNC_TIMEOUT_SECS: u64 = 5;
const HANDSHAKE_SEND_PERIOD_SECS: u64 = 5;
const STATUS_BEACON_TIMEOUT_SECS: u64 = 5;
const COMMAND_RESPONSE_TIMEOUT_SECS: u64 = 5;

struct PendingCommands {
    command_deadlines: VecDeque<CommandDeadline>,
}

impl PendingCommands {
    pub fn new() -> Self {
        Self {
            command_deadlines: VecDeque::new(),
        }
    }

    pub fn add(&mut self, cd: CommandDeadline) {
        self.command_deadlines.push_back(cd);
    }

    pub fn remove(&mut self, packet_type: PacketTypes) -> Result<()> {
        for (idx, ref cd) in self.command_deadlines.iter().enumerate() {
            if packet_type == cd.packet_type {
                self.command_deadlines.remove(idx);
                return Ok(());
            }
        }
        Err(anyhow!("Command not found: {:?}", packet_type))
    }

    pub async fn wait_for_timeout(&self) -> PacketTypes {
        if let Some(cd) = self.command_deadlines.front() {
            sleep_until(cd.deadline).await;
            return cd.packet_type;
        }
        // if there are no pending commands, just wait forever
        future::pending().await
    }
}

struct CommandDeadline {
    packet_type: PacketTypes,
    deadline: Instant,
}

impl CommandDeadline {
    pub fn new(packet_type: PacketTypes, dur: Duration) -> Self {
        Self {
            packet_type,
            deadline: Instant::now() + dur,
        }
    }
}

async fn run_instrument_comm_internal(
    tx_ic_to_iio: Sender<Vec<u8>>,
    mut rx_iio_to_ic: Receiver<Vec<u8>>,
    mut rx_ui_to_ic: Receiver<InstrumentCommand>,
) -> Result<()> {
    let mut tx_ic_to_fw: Option<Sender<ParsedMagnetometerData>> = None;
    let mut handshake_send_deadline =
        Instant::now() + Duration::from_secs(HANDSHAKE_SEND_PERIOD_SECS);

    let mut pending_commands = PendingCommands::new();
    pending_commands.add(CommandDeadline::new(
        PacketTypes::StatusBeacon,
        Duration::from_secs(STATUS_BEACON_TIMEOUT_SECS),
    ));

    // wait for packet sync, IIO will send just the magic word, so just need to init the buf and
    // nothing else
    let mut buf = match rx_iio_to_ic.recv().await {
        None => return Ok(()),
        Some(buf) => buf,
    };

    loop {
        tokio::select! {
            data = rx_iio_to_ic.recv() => {
                match data {
                    None => break,
                    Some(data) => {
                        buf.extend(data);
                        buf = handle_data_stream(buf, &mut pending_commands, &mut tx_ic_to_fw).await.with_context(|| "error handling data stream")?;
                    },
                }
            }
            command = rx_ui_to_ic.recv() => {
                match command {
                    None => break,
                    Some(command) => handle_command(command, &mut pending_commands, &tx_ic_to_iio, &mut tx_ic_to_fw).await?,
                }
            }
            _ = sleep_until(handshake_send_deadline) => {
                send_packet(&tx_ic_to_iio, OutgoingPacketPayloads::Handshake).await?;
                handshake_send_deadline = Instant::now() + Duration::from_secs(HANDSHAKE_SEND_PERIOD_SECS);
            },
            res = pending_commands.wait_for_timeout() => {
                bail!("Response timeout: {:?}", res);
            }
        }
    }

    Ok(())
}

async fn handle_command(
    command: InstrumentCommand,
    pending_commands: &mut PendingCommands,
    tx_ic_to_iio: &Sender<Vec<u8>>,
    tx_ic_to_fw: &mut Option<Sender<ParsedMagnetometerData>>,
) -> Result<()> {
    let payload = match command {
        InstrumentCommand::StartDataStream => Some(OutgoingPacketPayloads::StartDataStreaming),
        InstrumentCommand::StopDataStream => Some(OutgoingPacketPayloads::StopDataStreaming),
        InstrumentCommand::StartRecording(new_tx_ic_to_fw) => {
            *tx_ic_to_fw = Some(new_tx_ic_to_fw);
            None
        }
    };

    if let Some(payload) = payload {
        let packet_type = payload.packet_type();
        send_packet(tx_ic_to_iio, payload).await?;
        pending_commands.add(CommandDeadline::new(
            packet_type,
            Duration::from_secs(COMMAND_RESPONSE_TIMEOUT_SECS),
        ));
    }
    Ok(())
}

// TODO set this up to handle errors in status codes
async fn handle_data_stream(
    buf: Vec<u8>,
    pending_commands: &mut PendingCommands,
    tx_ic_to_fw: &mut Option<Sender<ParsedMagnetometerData>>,
) -> Result<Vec<u8>> {
    let res = parse_data_stream(buf)?;
    for packet in res.packets {
        if !matches!(packet.payload, IncomingPacketPayloads::MagnetometerData(_)) {
            println!("RECV: {:?}", packet);
        }
        // default to tracking the command. If it does not need to be tracked, the match arm will
        // set this to None
        let mut packet_type = Some(packet.payload.packet_type());
        match packet.payload {
            IncomingPacketPayloads::StatusBeacon(_) => {
                // TODO process the status codes
            }
            IncomingPacketPayloads::MagnetometerData(mag_data_packet) => {
                if let Some(ref tx_ic_to_fw_ref) = tx_ic_to_fw {
                    let send_res = tx_ic_to_fw_ref.send(mag_data_packet).await;
                    if send_res.is_err() {
                        // TODO how should this be handled? Should a sentinel msg be sent? Should
                        // this just assume that if the rx was dropped that the FW task finished
                        // recording / terminated?
                        println!("rx_ic_to_fw drop caused send to fail");
                        *tx_ic_to_fw = None;
                    }
                }
                packet_type = None;
            }
            IncomingPacketPayloads::Reboot => {}
            IncomingPacketPayloads::CheckConnectionStatus => {}
            IncomingPacketPayloads::Handshake(_) => {
                // TODO process the status codes

                // for the purpose of command tracking, a handshake response also counts as a status beacon
                packet_type = Some(PacketTypes::StatusBeacon);
            }
            IncomingPacketPayloads::PlateEvent => {}
            IncomingPacketPayloads::GoingDormant => {}
            IncomingPacketPayloads::SetStimProtocol => {}
            IncomingPacketPayloads::StartStim => {}
            IncomingPacketPayloads::StopStim => {}
            IncomingPacketPayloads::StimStatus => {}
            IncomingPacketPayloads::StimImpedanceCheck => {}
            IncomingPacketPayloads::InitOfflineMode => {}
            IncomingPacketPayloads::EndOfflineMode => {}
            IncomingPacketPayloads::SetSamplingPeriod => {}
            IncomingPacketPayloads::StartDataStreaming => {}
            IncomingPacketPayloads::StopDataStreaming => {}
            IncomingPacketPayloads::GetMetadata => {}
            IncomingPacketPayloads::SetNickname => {}
            IncomingPacketPayloads::BeginFirmwareUpdate => {}
            IncomingPacketPayloads::FirmwareUpdate => {}
            IncomingPacketPayloads::EndFirmwareUpdate => {}
            IncomingPacketPayloads::ChannelFirmwareUpdateComplete => {}
            IncomingPacketPayloads::MainFirmwareUpdateComplete => {}
            IncomingPacketPayloads::BarcodeFound => {}
            IncomingPacketPayloads::TriggerError => {}
            IncomingPacketPayloads::GetErrorDetails => {}
            IncomingPacketPayloads::ErrorAck => {}
            IncomingPacketPayloads::ChecksumFailure => {}
        }
        if let Some(packet_type) = packet_type {
            pending_commands.remove(packet_type)?;
            // status beacon deadline is not tied to a command, so it must be added back here
            if packet_type == PacketTypes::StatusBeacon {
                pending_commands.add(CommandDeadline::new(
                    packet_type,
                    Duration::from_secs(STATUS_BEACON_TIMEOUT_SECS),
                ));
            }
        }
    }
    return Ok(res.unread_bytes);
}

async fn send_packet(
    tx_ic_to_iio: &Sender<Vec<u8>>,
    payload: OutgoingPacketPayloads,
) -> Result<()> {
    let packet = OutgoingPacket::new(payload)?;
    println!("SEND: {:?}", packet);
    tx_ic_to_iio
        .send(packet.to_bytes())
        .await
        .with_context(|| "Error sending msg from IC to IIO")
}

async fn instrument_io(
    tx_iio_to_ic: Sender<Vec<u8>>,
    mut rx_ic_to_iio: Receiver<Vec<u8>>,
) -> Result<()> {
    let mut stream = TcpStream::connect("localhost:56575").await?;
    let (rx_from_ma, tx_to_ma) = stream.split();

    let instrument_io = InstrumentIO::new(rx_from_ma, tx_to_ma);

    let sync_timeout = Duration::from_secs(PACKET_SYNC_TIMEOUT_SECS);
    let buf = timeout(sync_timeout, instrument_io.sync_with_packets())
        .await
        .with_context(|| "Timeout waiting for packet sync")?
        .with_context(|| "Error syncing with packets")?;

    if let Err(_) = tx_iio_to_ic.send(buf).await {
        return Ok(());
    }

    loop {
        tokio::select! {
            res = rx_ic_to_iio.recv() => {
                match res {
                    None => break,
                    Some(buf) => instrument_io.write(buf).await.with_context(|| "Error writing to instrument")?,
                }
            }
            res = instrument_io.read_all() => {
                let buf = res.with_context(|| "Error reading from instrument")?;
                if let Err(_) = tx_iio_to_ic.send(buf).await {
                    break;
                }
            }
        }
    }

    Ok(())
}

struct InstrumentIO<'a> {
    reader: ReadHalf<'a>,
    writer: WriteHalf<'a>,
    max_read_size: usize,
}

impl<'a> InstrumentIO<'a> {
    fn new(reader: ReadHalf<'a>, writer: WriteHalf<'a>) -> Self {
        // TODO implement some logic to determine what this value should be?
        let max_read_size = 12500;
        Self {
            reader,
            writer,
            max_read_size,
        }
    }

    async fn read_all(&self) -> Result<Vec<u8>> {
        self.read(self.max_read_size).await
    }

    async fn read(&self, size: usize) -> Result<Vec<u8>> {
        let mut read_bytes = Vec::with_capacity(size);

        loop {
            // TODO handle socket disconnect?

            self.reader.readable().await?;

            return match self.reader.try_read_buf(&mut read_bytes) {
                Ok(0) => Err(anyhow!("0 bytes read")), // return an error indicating that the socket is disconnected?
                Ok(n) => {
                    read_bytes.resize(n, 0);
                    // println!("read {} bytes: {:?}", n, read_bytes);
                    Ok(read_bytes)
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => Err(e.into()),
            };
        }
    }

    async fn write(&self, msg: Vec<u8>) -> Result<()> {
        loop {
            self.writer.writable().await?;

            // Try to write data, this may still fail with `WouldBlock`
            // if the readiness event is a false positive.
            return match self.writer.try_write(&msg) {
                // TODO does 0 bytes sent need to handled differently?
                Ok(n) => {
                    // TODO if the whole packet wasn't written, try to write the remaining bytes or
                    // just return error?
                    // println!("wrote {}/{} bytes", n, packet.len());
                    Ok(())
                }
                Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
                    continue;
                }
                Err(e) => {
                    return Err(e.into());
                }
            };
        }
    }

    async fn sync_with_packets(&self) -> Result<Vec<u8>> {
        println!("Initiating packet sync");
        self.write(OutgoingPacket::new(OutgoingPacketPayloads::Handshake)?.to_bytes())
            .await?;

        let mut magic_word_buf = self.read(MAGIC_WORD_LEN).await?;

        while magic_word_buf.len() < MAGIC_WORD_LEN {
            let num_missing_bytes = MAGIC_WORD_LEN - magic_word_buf.len();
            let new_bytes = self.read(num_missing_bytes).await?;
            magic_word_buf.extend(new_bytes);
        }

        while !is_magic_word(magic_word_buf[..MAGIC_WORD_LEN].try_into().unwrap()) {
            magic_word_buf.remove(0);
            let new_byte = self.read(1).await?;
            magic_word_buf.extend(new_byte);
        }

        println!("Synced");
        Ok(magic_word_buf)
    }
}

// struct InstrumentComm<'a> {
//     reader: InstrumentReader<'a>,
//     writer: WriteHalf<'a>,
//     tx_ic_to_fw: Mutex<Option<Sender<ParsedMagnetometerData>>>,
// }

// impl<'a> InstrumentComm<'a> {
//     async fn new(stream: &'a mut TcpStream) -> Self {
//         let (read_half, write_half) = stream.split();
//         Self {
//             reader: InstrumentReader::new(read_half),
//             writer: write_half,
//             tx_ic_to_fw: Mutex::new(None),
//         }
//     }
//
//     async fn run(&mut self, command_rx: Receiver<InstrumentCommand>) -> Result<()> {
//         self.send_handshake().await?;
//
//         // TODO make a const for this timeout value
//         let buf = timeout(Duration::from_secs(5), self.sync_with_packets())
//             .await
//             // TODO make sure these error msgs are in the right order
//             .with_context(|| "Timeout waiting for packet sync")?
//             .with_context(|| "Error syncing with packets")?;
//
//         println!("SYNCED");
//
//         tokio::select! {
//             res = self.handle_data_stream(buf) => {
//                 res.with_context(|| "Error handling data stream")?
//             },
//             res = self.handle_commands(command_rx) => {
//                 res.with_context(|| "Error handling data stream")?
//             },
//             _ = self.handle_sending_handshakes() => {},
//         }
//
//         Ok(())
//     }
//
//     async fn sync_with_packets(&self) -> Result<Vec<u8>> {
//         let mut magic_word_buf = self.reader.read(MAGIC_WORD_LEN).await?;
//
//         while magic_word_buf.len() < MAGIC_WORD_LEN {
//             let num_missing_bytes = MAGIC_WORD_LEN - magic_word_buf.len();
//             let new_bytes = self.reader.read(num_missing_bytes).await?;
//             magic_word_buf.extend(new_bytes);
//         }
//
//         while !is_magic_word(magic_word_buf[..MAGIC_WORD_LEN].try_into().unwrap()) {
//             magic_word_buf.remove(0);
//             let new_byte = self.reader.read(1).await?;
//             magic_word_buf.extend(new_byte);
//         }
//
//         Ok(magic_word_buf)
//     }
//
//     async fn handle_commands(&self, mut rx: Receiver<InstrumentCommand>) -> Result<()> {
//         loop {
//             if let Some(msg) = rx.recv().await {
//                 match msg {
//                     InstrumentCommand::StartDataStream => {
//                         self.send_packet(OutgoingPacketPayloads::StartDataStreaming)
//                             .await?;
//                     }
//                     InstrumentCommand::StopDataStream => {
//                         self.send_packet(OutgoingPacketPayloads::StopDataStreaming)
//                             .await?;
//                     }
//                     InstrumentCommand::StartRecording(tx_ic_to_fw) => {
//                         *self.tx_ic_to_fw.lock().await = Some(tx_ic_to_fw);
//                     }
//                 }
//             }
//         }
//     }
//
//     async fn handle_data_stream(&self, mut buf: Vec<u8>) -> Result<()> {
//         let mut mag_data_stream_buf: Vec<ParsedMagnetometerData> = Vec::new();
//
//         loop {
//             buf.extend(self.reader.read_all().await?);
//             let res = parse_data_stream(buf)?; // TODO log the whole packet
//             for packet in res.packets {
//                 // TODO actually handle the packet
//                 println!("RECV: {:?}", packet);
//                 match packet.payload {
//                     IncomingPacketPayloads::MagnetometerData(mag_data_packet) => {
//                         // TODO buffer t
//                         let tx_ic_to_fw = self.tx_ic_to_fw.lock().await;
//                         if let Some(tx_ic_to_fw) = &*tx_ic_to_fw {
//                             let send_res = tx_ic_to_fw.send(mag_data_packet).await;
//                             if send_res.is_err() {}
//                         }
//                     }
//                     // TODO clear mag_data_stream_buf when StopDataStream response is received
//                     _ => {}
//                 }
//             }
//             buf = res.unread_bytes;
//         }
//     }
//
//     async fn handle_sending_handshakes(&self) -> Result<()> {
//         loop {
//             // TODO make a constant for this
//             sleep(Duration::from_secs(5)).await;
//             self.send_handshake().await?
//         }
//     }
//
//     async fn send_handshake(&self) -> Result<()> {
//         self.send_packet(OutgoingPacketPayloads::Handshake).await
//     }
//
//     async fn send_packet(&self, payload: OutgoingPacketPayloads) -> Result<()> {
//         // TODO Acquire a lock here?
//
//         let packet = OutgoingPacket::new(payload)?;
//         println!("SEND: {:?}", packet);
//         let packet = packet.to_bytes();
//
//         loop {
//             self.writer.writable().await?;
//
//             // Try to write data, this may still fail with `WouldBlock`
//             // if the readiness event is a false positive.
//             return match self.writer.try_write(&packet) {
//                 // TODO does 0 bytes sent need to handled differently?
//                 Ok(n) => {
//                     // TODO if the whole packet wasn't written, try to write the remaining bytes or
//                     // just return error?
//                     // println!("wrote {}/{} bytes", n, packet.len());
//                     Ok(())
//                 }
//                 Err(ref e) if e.kind() == io::ErrorKind::WouldBlock => {
//                     continue;
//                 }
//                 Err(e) => {
//                     return Err(e.into());
//                 }
//             };
//         }
//     }
// }
//
