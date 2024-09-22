use anyhow::{Context, Result};
use core::fmt;
use deku::{DekuContainerRead, DekuContainerWrite, DekuRead, DekuWrite};
use std::mem::size_of;

const NUM_WELLS: usize = 24; // TODO move this somewhere else?

#[derive(Debug, DekuRead, PartialEq)]
#[deku(id_type = "u8")]
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

// TODO try to write a macro to derive this for all the payloads
trait WriteTypedPayload: DekuContainerWrite + fmt::Debug {
    fn packet_type() -> PacketTypes;

    fn write(&self) -> Result<(PacketTypes, Vec<u8>)> {
        let packet_type = Self::packet_type();
        let payload = self
            .to_bytes()
            .with_context(|| format!("failed to convert payload to bytes {:?}", self))?;

        Ok((packet_type, payload))
    }
}

#[derive(Debug, DekuWrite, DekuRead)]
struct StatusBeacon {
    pub main_status: u8,
    pub index_of_thread_with_error: u8,
    pub module_statuses: [u8; NUM_WELLS],
}

impl WriteTypedPayload for StatusBeacon {
    fn packet_type() -> PacketTypes {
        PacketTypes::StatusBeacon
    }
}

pub const MAGIC_WORD_LEN: usize = 8;
pub type MagicWordBuf = [u8; MAGIC_WORD_LEN];
pub const MAGIC_WORD: MagicWordBuf = *b"CURI BIO";
#[derive(Debug, DekuRead, PartialEq)]
struct PacketHeader {
    magic_word: MagicWordBuf,
    packet_remainder_size: u16,
}

// TODO try writing a macro to define the packed size of a struct, or see if one exists
impl PacketHeader {
    fn new(payload_len: usize) -> Result<Self> {
        let packet_remainder_size = PACKET_BASE_LEN + payload_len + CHECKSUM_LEN;
        let packet_remainder_size: u16 = packet_remainder_size
            .try_into()
            .with_context(|| format!("Invalid packet remainder size: {}", packet_remainder_size))?;

        Ok(Self {
            magic_word: MAGIC_WORD.clone(),
            packet_remainder_size,
        })
    }
}

const TIMESTAMP_LEN: usize = size_of::<u64>();
const PACKET_TYPE_LEN: usize = size_of::<u8>();
const PACKET_BASE_LEN: usize = TIMESTAMP_LEN + PACKET_TYPE_LEN;
#[derive(Debug, DekuRead, PartialEq)]
struct PacketBase {
    timestamp: u64,
    packet_type: PacketTypes,
}

impl PacketBase {
    fn new(packet_type: PacketTypes) -> Self {
        Self {
            timestamp: 0, // TODO set to the current time
            packet_type,
        }
    }
}

const CHECKSUM_LEN: usize = size_of::<u32>();
const REMAINDER_EXCLUDING_PAYLOAD: usize = TIMESTAMP_LEN + PACKET_TYPE_LEN + CHECKSUM_LEN;
#[derive(Debug, DekuRead, PartialEq)]
struct Packet {
    header: PacketHeader,
    base: PacketBase,
    // TODO let this be option?
    #[deku(count = "header.packet_remainder_size - REMAINDER_EXCLUDING_PAYLOAD as u16")]
    payload: Vec<u8>, /* intentionally not using a struct for this since payloads are converted to/from bytes
                      separately from packets */
    checksum: u32,
}

impl Packet {
    fn new(payload: impl WriteTypedPayload) -> Result<Self> {
        let (packet_type, payload) = payload.write()?;

        Ok(Self {
            header: PacketHeader::new(payload.len())?,
            base: PacketBase::new(packet_type),
            payload,
            checksum: 0, // TODO try to let deku handle this
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity() {
        let packet = Packet::new(StatusBeacon {
            main_status: 100,
            index_of_thread_with_error: 101,
            module_statuses: [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23,
            ],
        })
        .expect("failed to write packet");
        // let res = packet.to_bytes();
        let expected_bytes = [
            67, 85, 82, 73, 32, 66, 73, 79, 39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 101, 0, 1, 2, 3,
            4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 0, 0, 0, 0,
        ];
        // assert_eq!(res, expected_bytes);
        let res = Packet::from_bytes((&expected_bytes, 0))
            .expect("failed to read packet")
            .1;
        assert_eq!(res, packet);
    }
}
