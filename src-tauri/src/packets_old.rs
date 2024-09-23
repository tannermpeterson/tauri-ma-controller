use std::mem::size_of;

use anyhow::{anyhow, bail, Context, Result};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

pub const MAGIC_WORD_LEN: usize = 8;
pub type MagicWordBuf = [u8; MAGIC_WORD_LEN];
pub const MAGIC_WORD: MagicWordBuf = *b"CURI BIO";
const PACKET_REMAINDER_LEN: usize = size_of::<u16>();
const PACKET_HEADER_LEN: usize = MAGIC_WORD_LEN + PACKET_REMAINDER_LEN;
const TIMESTAMP_LEN: usize = size_of::<u64>();
const PACKET_TYPE_LEN: usize = size_of::<u8>();
const PACKET_BASE_LEN: usize = TIMESTAMP_LEN + PACKET_TYPE_LEN;
const CHECKSUM_LEN: usize = size_of::<u32>();
const MIN_PACKET_LEN: usize = PACKET_HEADER_LEN + PACKET_BASE_LEN + CHECKSUM_LEN;

const NUM_WELLS: usize = 24; // TODO move this somewhere else?

#[derive(FromPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PacketTypes {
    // General
    StatusBeacon = 0,
    MagnetometerData = 1,
    Reboot = 2,
    CheckConnectionStatus = 3,
    Handshake = 4,
    PlateEvent = 6,
    GoingDormant = 10,
    // Stimulation
    SetStimProtocol = 20,
    StartStim = 21,
    StopStim = 22,
    StimStatus = 23,
    StimImpedanceCheck = 27,
    // offline mode
    InitOfflineMode = 40,
    EndOfflineMode = 41,
    // Magnetometer
    SetSamplingPeriod = 50,
    StartDataStreaming = 52,
    StopDataStreaming = 53,
    // Metadata
    GetMetadata = 60,
    SetNickname = 62,
    // Firmware Updating
    BeginFirmwareUpdate = 70,
    FirmwareUpdate = 71,
    EndFirmwareUpdate = 72,
    ChannelFirmwareUpdateComplete = 73,
    MainFirmwareUpdateComplete = 74,
    // Barcode
    BarcodeFound = 90,
    // Misc?
    TriggerError = 103,
    // Errors
    GetErrorDetails = 253,
    ErrorAck = 254,
    ChecksumFailure = 255,
}

fn get_timestamp() -> u64 {
    0 // TODO
}

fn get_packet_remainder_size(payload: &Option<Vec<u8>>) -> Result<u16> {
    let mut size = PACKET_BASE_LEN + CHECKSUM_LEN;
    if let Some(payload) = payload {
        size += payload.len();
    }
    size.try_into()
        .with_context(|| format!("Invalid packet remainder size: {}", size))
}

pub fn is_magic_word(buf: &MagicWordBuf) -> bool {
    for i in 0..MAGIC_WORD_LEN {
        if buf[i] != MAGIC_WORD[i] {
            return false;
        }
    }
    true
}

#[derive(Debug)]
pub struct OutgoingPacket {
    packet_header: PacketHeader,
    packet_base: PacketBase,
    payload: Option<Vec<u8>>,
}

impl OutgoingPacket {
    pub fn new(payload: OutgoingPacketPayloads) -> Result<Self> {
        let packet_type = payload.packet_type();
        let packet_base = PacketBase::new(packet_type);
        let payload = payload.to_bytes();
        let packet_remainder_size = get_packet_remainder_size(&payload)
            .with_context(|| format!("Packet Type: {:?}, Payload: {:?}", packet_type, payload))?;
        let packet_header = PacketHeader::new(packet_remainder_size);

        Ok(Self {
            packet_header,
            packet_base,
            payload,
        })
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut packet_bytes = self.packet_header.to_bytes();
        packet_bytes.extend(self.packet_base.to_bytes());
        if let Some(payload) = self.payload {
            packet_bytes.extend(payload);
        }
        packet_bytes.extend([0; CHECKSUM_LEN]); // TODO add the actual checksum

        packet_bytes
    }
}

#[derive(Debug)]
struct PacketHeader {
    magic_word: MagicWordBuf,
    packet_remainder_size: u16,
}

impl PacketHeader {
    fn new(packet_remainder_size: u16) -> Self {
        Self {
            magic_word: MAGIC_WORD.clone(),
            packet_remainder_size,
        }
    }

    fn to_bytes(self) -> Vec<u8> {
        self.magic_word
            .into_iter()
            .chain(self.packet_remainder_size.to_le_bytes().into_iter())
            .collect::<Vec<u8>>()
    }
}

// TODO see if this would be nice or not https://docs.rs/packed_struct/latest/packed_struct/
#[derive(Debug)]
struct PacketBase {
    timestamp: u64,
    packet_type: PacketTypes,
}

impl PacketBase {
    fn new(packet_type: PacketTypes) -> Self {
        Self {
            timestamp: get_timestamp(),
            packet_type,
        }
    }

    fn to_bytes(self) -> Vec<u8> {
        let mut bytes = self.timestamp.to_le_bytes().to_vec();
        bytes.push(self.packet_type as u8);
        bytes
    }
}

fn parse_packet_type(packet_type: u8) -> Result<PacketTypes> {
    match FromPrimitive::from_u8(packet_type) {
        Some(pt) => Ok(pt),
        None => Err(anyhow!("Invalid packet type: {}", packet_type)),
    }
}

#[derive(Debug)]
pub enum OutgoingPacketPayloads {
    StatusBeacon,
    Reboot {},
    CheckConnectionStatus {},
    Handshake,
    SetStimProtocol {},
    StartStim {},
    StopStim {},
    StimImpedanceCheck {},
    InitOfflineMode {},
    EndOfflineMode {},
    SetSamplingPeriod {},
    StartDataStreaming,
    StopDataStreaming,
    GetMetadata {},
    SetNickname {},
    BeginFirmwareUpdate {},
    FirmwareUpdate {},
    EndFirmwareUpdate {},
    TriggerError {},
    GetErrorDetails {},
    ErrorAck {},
}

impl OutgoingPacketPayloads {
    fn to_bytes(self) -> Option<Vec<u8>> {
        match self {
            _ => None,
        }
    }

    // TODO is there a better way of doing this?
    pub fn packet_type(&self) -> PacketTypes {
        match self {
            Self::StatusBeacon { .. } => PacketTypes::StatusBeacon,
            Self::Reboot { .. } => PacketTypes::Reboot,
            Self::CheckConnectionStatus { .. } => PacketTypes::CheckConnectionStatus,
            Self::Handshake => PacketTypes::Handshake,
            Self::SetStimProtocol { .. } => PacketTypes::SetStimProtocol,
            Self::StartStim { .. } => PacketTypes::StartStim,
            Self::StopStim { .. } => PacketTypes::StopStim,
            Self::StimImpedanceCheck { .. } => PacketTypes::StimImpedanceCheck,
            Self::InitOfflineMode { .. } => PacketTypes::InitOfflineMode,
            Self::EndOfflineMode { .. } => PacketTypes::EndOfflineMode,
            Self::SetSamplingPeriod { .. } => PacketTypes::SetSamplingPeriod,
            Self::StartDataStreaming => PacketTypes::StartDataStreaming,
            Self::StopDataStreaming => PacketTypes::StopDataStreaming,
            Self::GetMetadata { .. } => PacketTypes::GetMetadata,
            Self::SetNickname { .. } => PacketTypes::SetNickname,
            Self::BeginFirmwareUpdate { .. } => PacketTypes::BeginFirmwareUpdate,
            Self::FirmwareUpdate { .. } => PacketTypes::FirmwareUpdate,
            Self::EndFirmwareUpdate { .. } => PacketTypes::EndFirmwareUpdate,
            Self::TriggerError { .. } => PacketTypes::TriggerError,
            Self::GetErrorDetails { .. } => PacketTypes::GetErrorDetails,
            Self::ErrorAck { .. } => PacketTypes::ErrorAck,
        }
    }
}

#[derive(Debug)]
pub struct IncomingPacket {
    pub timestamp: u64,
    pub payload: IncomingPacketPayloads,
}

#[derive(Debug)]
pub enum IncomingPacketPayloads {
    StatusBeacon(StatusCodes),
    MagnetometerData(ParsedMagnetometerData),
    Reboot,
    CheckConnectionStatus,
    Handshake(StatusCodes),
    PlateEvent,
    GoingDormant,
    SetStimProtocol,
    StartStim,
    StopStim,
    StimStatus,
    StimImpedanceCheck,
    InitOfflineMode,
    EndOfflineMode,
    SetSamplingPeriod,
    StartDataStreaming,
    StopDataStreaming,
    GetMetadata,
    SetNickname,
    BeginFirmwareUpdate,
    FirmwareUpdate,
    EndFirmwareUpdate,
    ChannelFirmwareUpdateComplete,
    MainFirmwareUpdateComplete,
    BarcodeFound,
    TriggerError,
    GetErrorDetails,
    ErrorAck,
    ChecksumFailure,
}

impl IncomingPacketPayloads {
    // TODO is there a better way of doing this?
    pub fn packet_type(&self) -> PacketTypes {
        match self {
            Self::StatusBeacon(_) => PacketTypes::StatusBeacon,
            Self::MagnetometerData(_) => PacketTypes::MagnetometerData,
            Self::Reboot => PacketTypes::Reboot,
            Self::CheckConnectionStatus => PacketTypes::CheckConnectionStatus,
            Self::Handshake(_) => PacketTypes::Handshake,
            Self::PlateEvent => PacketTypes::PlateEvent,
            Self::GoingDormant => PacketTypes::GoingDormant,
            Self::SetStimProtocol => PacketTypes::SetStimProtocol,
            Self::StartStim => PacketTypes::StartStim,
            Self::StopStim => PacketTypes::StopStim,
            Self::StimStatus => PacketTypes::StimStatus,
            Self::StimImpedanceCheck => PacketTypes::StimImpedanceCheck,
            Self::InitOfflineMode => PacketTypes::InitOfflineMode,
            Self::EndOfflineMode => PacketTypes::EndOfflineMode,
            Self::SetSamplingPeriod => PacketTypes::SetSamplingPeriod,
            Self::StartDataStreaming => PacketTypes::StartDataStreaming,
            Self::StopDataStreaming => PacketTypes::StopDataStreaming,
            Self::GetMetadata => PacketTypes::GetMetadata,
            Self::SetNickname => PacketTypes::SetNickname,
            Self::BeginFirmwareUpdate => PacketTypes::BeginFirmwareUpdate,
            Self::FirmwareUpdate => PacketTypes::FirmwareUpdate,
            Self::EndFirmwareUpdate => PacketTypes::EndFirmwareUpdate,
            Self::ChannelFirmwareUpdateComplete => PacketTypes::ChannelFirmwareUpdateComplete,
            Self::MainFirmwareUpdateComplete => PacketTypes::MainFirmwareUpdateComplete,
            Self::BarcodeFound => PacketTypes::BarcodeFound,
            Self::TriggerError => PacketTypes::TriggerError,
            Self::GetErrorDetails => PacketTypes::GetErrorDetails,
            Self::ErrorAck => PacketTypes::ErrorAck,
            Self::ChecksumFailure => PacketTypes::ChecksumFailure,
        }
    }
}

// TODO if parsing is slow, change the with_contexts to expects and figure out how to handle the
// panics correctly. OR figure out how to remove the try_intos for something better (Try deku)
#[derive(Debug)]
pub struct StatusCodes {
    pub main_status: u8,
    pub index_of_thread_with_error: u8,
    pub module_statuses: [u8; NUM_WELLS],
}

impl StatusCodes {
    fn from_bytes(payload: &[u8]) -> Result<Self> {
        let payload: [u8; NUM_WELLS + 2] = payload
            .try_into()
            .with_context(|| format!("Incorrect length for status codes: {}", payload.len()))?;
        Ok(Self {
            main_status: payload[0],
            index_of_thread_with_error: payload[1],
            module_statuses: payload[2..NUM_WELLS + 2]
                .try_into()
                .with_context(|| "Incorrect length for module statuses")?,
        })
    }
}

const TIME_IDX_LEN: usize = size_of::<u64>();
const NUM_AXES: usize = 3;
const NUM_SENSORS: usize = 3;
const TIME_OFFSET_LEN: usize = size_of::<u16>();
const SENSOR_DATA_LEN: usize = TIME_OFFSET_LEN + NUM_AXES * size_of::<u16>();
const WELL_DATA_LEN: usize = SENSOR_DATA_LEN * NUM_SENSORS;
const MAGNETOMETER_DATA_PAYLOAD_LEN: usize =
    TIME_IDX_LEN + NUM_WELLS * NUM_SENSORS * SENSOR_DATA_LEN;

// TODO parse all wells
#[derive(Debug)]
pub struct ParsedMagnetometerData {
    pub time_index: u64,
    pub well_data: [WellData; NUM_WELLS],
}

impl ParsedMagnetometerData {
    fn from_bytes(payload: &[u8]) -> Result<Self> {
        let payload: [u8; MAGNETOMETER_DATA_PAYLOAD_LEN] =
            payload.try_into().with_context(|| {
                format!("Incorrect length for magnetometer data: {}", payload.len())
            })?;
        let (time_index_buf, well_data_buf) = payload.split_at(TIME_IDX_LEN);
        let time_index = u64::from_le_bytes(
            time_index_buf
                .try_into()
                .with_context(|| "Incorrect length for magnetometer data")?,
        );

        let mut well_data = Vec::new();
        for well_idx in 0..NUM_WELLS {
            well_data.push(WellData::from_bytes(
                well_data_buf[WELL_DATA_LEN * well_idx..WELL_DATA_LEN * (well_idx + 1)]
                    .try_into()
                    .with_context(|| format!("Incorrect length for well {} data", well_idx))?,
            )?)
        }

        // TODO not sure why with_context isn't working here
        let well_data = well_data
            .try_into()
            .expect("Error converting well data vec to array");

        Ok(Self {
            time_index,
            well_data,
        })
    }
}

#[derive(Debug)]
pub struct WellData {
    pub s1: SensorData,
    pub s2: SensorData,
    pub s3: SensorData,
}

impl WellData {
    fn from_bytes(buf: &[u8; WELL_DATA_LEN]) -> Result<Self> {
        Ok(Self {
            s1: SensorData::from_bytes(
                buf[..SENSOR_DATA_LEN]
                    .try_into()
                    .with_context(|| "Incorrect length from sensor 1 data")?,
            )?,
            s2: SensorData::from_bytes(
                buf[SENSOR_DATA_LEN..SENSOR_DATA_LEN * 2]
                    .try_into()
                    .with_context(|| "Incorrect length from sensor 2 data")?,
            )?,
            s3: SensorData::from_bytes(
                buf[SENSOR_DATA_LEN * 2..SENSOR_DATA_LEN * 3]
                    .try_into()
                    .with_context(|| "Incorrect length from sensor 3 data")?,
            )?,
        })
    }
}

#[derive(Debug)]
pub struct SensorData {
    pub time_offset: u16,
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

impl SensorData {
    fn from_bytes(buf: &[u8; SENSOR_DATA_LEN]) -> Result<Self> {
        Ok(Self {
            time_offset: u16::from_le_bytes(
                buf[..2]
                    .try_into()
                    .with_context(|| "incorrect length for time offset")?,
            ),
            x: u16::from_le_bytes(
                buf[2..4]
                    .try_into()
                    .with_context(|| "incorrect length for axis x sample")?,
            ),
            y: u16::from_le_bytes(
                buf[4..6]
                    .try_into()
                    .with_context(|| "incorrect length for axis y sample")?,
            ),
            z: u16::from_le_bytes(
                buf[6..SENSOR_DATA_LEN]
                    .try_into()
                    .with_context(|| "incorrect length for axis z sample")?,
            ),
        })
    }
}

struct ParsedStimData {}

pub struct DataParseResult {
    pub unread_bytes: Vec<u8>,
    pub packets: Vec<IncomingPacket>,
}

pub fn parse_data_stream(buf: Vec<u8>) -> Result<DataParseResult> {
    let total_num_bytes = buf.len();
    let mut curr_idx: usize = 0;

    let mut packets = Vec::new();

    while curr_idx + MIN_PACKET_LEN < total_num_bytes {
        let magic_word_buf = &buf[..MAGIC_WORD_LEN];
        // TODO clean up all the expects?
        if !is_magic_word(
            magic_word_buf
                .try_into()
                .expect("Incorrect magic word length"),
        ) {
            bail!(
                "Invalid magic word at {} / {}: {:?}",
                curr_idx,
                total_num_bytes,
                magic_word_buf
            );
        }

        let packet_remainder_idx = curr_idx + MAGIC_WORD_LEN;
        let timestamp_idx = packet_remainder_idx + PACKET_REMAINDER_LEN;
        let num_bytes_remaining_in_packet: usize = u16::from_le_bytes(
            buf[packet_remainder_idx..timestamp_idx]
                .try_into()
                .expect("Incorrect packet remainder length"),
        )
        .into();
        let packet_type_idx = timestamp_idx + TIMESTAMP_LEN;
        let payload_idx = packet_type_idx + PACKET_TYPE_LEN;
        let packet_stop_idx = timestamp_idx + num_bytes_remaining_in_packet;
        let checksum_idx = packet_stop_idx - CHECKSUM_LEN;
        // the while condition is only sufficient to see if it is appropriate to begin parsing
        // another packet, this is also required to determine if this current packet can be
        // completely parsed
        if packet_stop_idx > total_num_bytes {
            break;
        }

        let err_msg = || {
            format!(
                "Bytes {}-{} / {}",
                curr_idx, packet_stop_idx, total_num_bytes
            )
        };

        let timestamp = u64::from_le_bytes(
            buf[timestamp_idx..packet_type_idx]
                .try_into()
                .expect("Incorrect timestamp len"),
        );
        let packet_type = parse_packet_type(buf[packet_type_idx]).with_context(err_msg)?;
        let payload = &buf[payload_idx..checksum_idx];
        let checksum = u32::from_le_bytes(
            buf[checksum_idx..packet_stop_idx]
                .try_into()
                .with_context(|| "Incorrect checksum len")?, // TODO could probably handle this better
        );
        // TODO validate checksum

        let payload = parse_payload(payload, packet_type).with_context(err_msg)?;

        packets.push(IncomingPacket { timestamp, payload });

        curr_idx = packet_stop_idx;
    }

    Ok(DataParseResult {
        unread_bytes: buf[curr_idx..].to_vec(),
        packets,
    })
}

fn parse_payload(payload: &[u8], packet_type: PacketTypes) -> Result<IncomingPacketPayloads> {
    let parsed_payload = match packet_type {
        PacketTypes::StatusBeacon => {
            IncomingPacketPayloads::StatusBeacon(StatusCodes::from_bytes(payload)?)
        }
        PacketTypes::MagnetometerData => {
            IncomingPacketPayloads::MagnetometerData(ParsedMagnetometerData::from_bytes(payload)?)
        }
        PacketTypes::Reboot => IncomingPacketPayloads::Reboot,
        PacketTypes::CheckConnectionStatus => IncomingPacketPayloads::CheckConnectionStatus,
        PacketTypes::Handshake => {
            IncomingPacketPayloads::Handshake(StatusCodes::from_bytes(payload)?)
        }
        PacketTypes::PlateEvent => IncomingPacketPayloads::PlateEvent,
        PacketTypes::GoingDormant => IncomingPacketPayloads::GoingDormant,
        PacketTypes::SetStimProtocol => IncomingPacketPayloads::SetStimProtocol,
        PacketTypes::StartStim => IncomingPacketPayloads::StartStim,
        PacketTypes::StopStim => IncomingPacketPayloads::StopStim,
        PacketTypes::StimStatus => IncomingPacketPayloads::StimStatus,
        PacketTypes::StimImpedanceCheck => IncomingPacketPayloads::StimImpedanceCheck,
        PacketTypes::InitOfflineMode => IncomingPacketPayloads::InitOfflineMode,
        PacketTypes::EndOfflineMode => IncomingPacketPayloads::EndOfflineMode,
        PacketTypes::SetSamplingPeriod => IncomingPacketPayloads::SetSamplingPeriod,
        PacketTypes::StartDataStreaming => IncomingPacketPayloads::StartDataStreaming,
        PacketTypes::StopDataStreaming => IncomingPacketPayloads::StopDataStreaming,
        PacketTypes::GetMetadata => IncomingPacketPayloads::GetMetadata,
        PacketTypes::SetNickname => IncomingPacketPayloads::SetNickname,
        PacketTypes::BeginFirmwareUpdate => IncomingPacketPayloads::BeginFirmwareUpdate,
        PacketTypes::FirmwareUpdate => IncomingPacketPayloads::FirmwareUpdate,
        PacketTypes::EndFirmwareUpdate => IncomingPacketPayloads::EndFirmwareUpdate,
        PacketTypes::ChannelFirmwareUpdateComplete => {
            IncomingPacketPayloads::ChannelFirmwareUpdateComplete
        }
        PacketTypes::MainFirmwareUpdateComplete => {
            IncomingPacketPayloads::MainFirmwareUpdateComplete
        }
        PacketTypes::BarcodeFound => IncomingPacketPayloads::BarcodeFound,
        PacketTypes::TriggerError => IncomingPacketPayloads::TriggerError,
        PacketTypes::GetErrorDetails => IncomingPacketPayloads::GetErrorDetails,
        PacketTypes::ErrorAck => IncomingPacketPayloads::ErrorAck,
        PacketTypes::ChecksumFailure => IncomingPacketPayloads::ChecksumFailure,
    };
    Ok(parsed_payload)
}

// #[cfg(test)]
// mod tests {
//     use super::*;
//
//     #[test]
//     fn sanity() {
//         let packet_type = 123;
//         let payload = vec![10, 20, 30];
//         let packet = Packet::with_payload(packet_type, payload);
//         let serialized = [
//             67, 85, 82, 73, 32, 66, 73, 79, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 123, 10, 20, 30, 0, 0,
//             0, 0,
//         ];
//         assert_eq!(to_bytes(&packet).unwrap(), serialized);
//         assert_eq!(from_bytes(&serialized), packet);
//     }
// }
