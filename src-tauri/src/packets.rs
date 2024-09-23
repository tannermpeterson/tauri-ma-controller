use anyhow::{bail, Context, Result};
use deku::{
    deku_derive, DekuContainerRead, DekuContainerWrite, DekuEnumExt, DekuError, DekuRead, DekuWrite,
};
use std::{fmt, mem::size_of};

const NUM_WELLS: usize = 24; // TODO move this somewhere else?

pub type PacketType = u8;

// TODO add tests to make sure that the IDs match for variants that are in both enums?
#[cfg_attr(test, derive(Clone))]
#[derive(Debug, DekuWrite, DekuRead, PartialEq)] // It seems that DekuRead is require to get the deku_id method
#[deku(id_type = "PacketType")]
pub enum OutgoingPacketPayloads {
    // General
    // #[deku(id = 2)]
    // Reboot(RebootPayload),
    // #[deku(id = 3)]
    // CheckConnectionStatus(CheckConnectionStatusPayload),
    #[deku(id = 4)]
    Handshake,
    // #[deku(id = 6)]
    // PlateEvent(PlateEventPayload),
    // #[deku(id = 10)]
    // GoingDormant(GoingDormantPayload),
    // // Stimulation
    // #[deku(id = 20)]
    // SetStimProtocol(SetStimProtocolPayload),
    // #[deku(id = 21)]
    // StartStim(StartStimPayload),
    // #[deku(id = 22)]
    // StopStim(StopStimPayload),
    // #[deku(id = 23)]
    // StimStatus(StimStatusPayload),
    // #[deku(id = 27)]
    // StimImpedanceCheck(StimImpedanceCheckPayload),
    // // offline mode
    // #[deku(id = 40)]
    // InitOfflineMode(InitOfflineModePayload),
    // #[deku(id = 41)]
    // EndOfflineMode(EndOfflineModePayload),
    // // Magnetometer
    // #[deku(id = 50)]
    // SetSamplingPeriod(SetSamplingPeriodPayload),
    #[deku(id = 52)]
    StartDataStreaming,
    #[deku(id = 53)]
    StopDataStreaming,
    // // Metadata
    // #[deku(id = 60)]
    // GetMetadata(GetMetadataPayload),
    // #[deku(id = 62)]
    // SetNickname(SetNicknamePayload),
    // // Firmware Updating
    // #[deku(id = 70)]
    // BeginFirmwareUpdate(BeginFirmwareUpdatePayload),
    // #[deku(id = 71)]
    // FirmwareUpdate(FirmwareUpdatePayload),
    // #[deku(id = 72)]
    // EndFirmwareUpdate(EndFirmwareUpdatePayload),
    // #[deku(id = 73)]
    // ChannelFirmwareUpdateComplete(ChannelFirmwareUpdateCompletePayload),
    // #[deku(id = 74)]
    // MainFirmwareUpdateComplete(MainFirmwareUpdateCompletePayload),
    // // Barcode
    // #[deku(id = 90)]
    // BarcodeFound(BarcodeFoundPayload),
    // // Misc?
    // #[deku(id = 103)]
    // TriggerError(TriggerErrorPayload),
    // // Errors
    // #[deku(id = 253)]
    // GetErrorDetails(GetErrorDetailsPayload),
    // #[deku(id = 254)]
    // ErrorAck(ErrorAckPayload),
    // #[deku(id = 255)]
    // ChecksumFailure(ChecksumFailurePayload),
}

impl OutgoingPacketPayloads {
    pub fn packet_type(&self) -> PacketType {
        // TODO see what happen if this expect fails in the full running app
        self.deku_id().expect("should have id assigned")
    }
}

// TODO add a test to make sure this matches the enum

pub const STATUS_BEACON_PACKET_TYPE: PacketType = 0;

#[cfg_attr(test, derive(Clone, DekuWrite))]
#[derive(Debug, DekuRead, PartialEq)]
#[deku(id_type = "PacketType")]
pub enum IncomingPacketPayloads {
    // General
    #[deku(id = 0)]
    StatusBeacon(IncomingStatusBeaconPayload),
    #[deku(id = 1)]
    MagnetometerData(IncomingMagnetometerDataPayload),
    // #[deku(id = 2)]
    // Reboot(RebootPayload),
    // #[deku(id = 3)]
    // CheckConnectionStatus(CheckConnectionStatusPayload),
    #[deku(id = 4)]
    Handshake(IncomingStatusBeaconPayload),
    // #[deku(id = 6)]
    // PlateEvent(PlateEventPayload),
    // #[deku(id = 10)]
    // GoingDormant(GoingDormantPayload),
    // // Stimulation
    // #[deku(id = 20)]
    // SetStimProtocol(SetStimProtocolPayload),
    // #[deku(id = 21)]
    // StartStim(StartStimPayload),
    // #[deku(id = 22)]
    // StopStim(StopStimPayload),
    // #[deku(id = 23)]
    // StimStatus(StimStatusPayload),
    // #[deku(id = 27)]
    // StimImpedanceCheck(StimImpedanceCheckPayload),
    // // offline mode
    // #[deku(id = 40)]
    // InitOfflineMode(InitOfflineModePayload),
    // #[deku(id = 41)]
    // EndOfflineMode(EndOfflineModePayload),
    // // Magnetometer
    // #[deku(id = 50)]
    // SetSamplingPeriod(SetSamplingPeriodPayload),
    #[deku(id = 52)]
    StartDataStreaming(IncomingStartDataStreamingPayload),
    #[deku(id = 53)]
    StopDataStreaming(bool),
    // // Metadata
    // #[deku(id = 60)]
    // GetMetadata(GetMetadataPayload),
    // #[deku(id = 62)]
    // SetNickname(SetNicknamePayload),
    // // Firmware Updating
    // #[deku(id = 70)]
    // BeginFirmwareUpdate(BeginFirmwareUpdatePayload),
    // #[deku(id = 71)]
    // FirmwareUpdate(FirmwareUpdatePayload),
    // #[deku(id = 72)]
    // EndFirmwareUpdate(EndFirmwareUpdatePayload),
    // #[deku(id = 73)]
    // ChannelFirmwareUpdateComplete(ChannelFirmwareUpdateCompletePayload),
    // #[deku(id = 74)]
    // MainFirmwareUpdateComplete(MainFirmwareUpdateCompletePayload),
    // // Barcode
    // #[deku(id = 90)]
    // BarcodeFound(BarcodeFoundPayload),
    // // Misc?
    // #[deku(id = 103)]
    // TriggerError(TriggerErrorPayload),
    // // Errors
    // #[deku(id = 253)]
    // GetErrorDetails(GetErrorDetailsPayload),
    // #[deku(id = 254)]
    // ErrorAck(ErrorAckPayload),
    // #[deku(id = 255)]
    // ChecksumFailure(ChecksumFailurePayload),
}

impl IncomingPacketPayloads {
    pub fn packet_type(&self) -> PacketType {
        // TODO see what happen if this expect fails in the full running app
        self.deku_id().expect("should have id assigned")
    }
}

#[cfg_attr(test, derive(Clone, DekuWrite))]
#[derive(Debug, DekuRead, PartialEq)]
pub struct IncomingStatusBeaconPayload {
    pub main_status: u8,
    pub index_of_thread_with_error: u8,
    pub module_statuses: [u8; NUM_WELLS],
}

#[cfg_attr(test, derive(Clone, DekuWrite))]
#[derive(Debug, DekuRead, PartialEq)]
pub struct IncomingMagnetometerDataPayload {
    pub time_index: u64,
    pub well_data: [WellData; NUM_WELLS],
}

#[cfg_attr(test, derive(Clone, DekuWrite))]
#[derive(Debug, DekuRead, PartialEq)]
pub struct SensorData {
    pub time_offset: u16,
    pub x: u16,
    pub y: u16,
    pub z: u16,
}

#[cfg_attr(test, derive(Clone, DekuWrite))]
#[derive(Debug, DekuRead, PartialEq)]
pub struct WellData {
    pub s1: SensorData,
    pub s2: SensorData,
    pub s3: SensorData,
}

#[cfg_attr(test, derive(Clone, DekuWrite))]
#[derive(Debug, DekuRead, PartialEq)]
pub struct IncomingStartDataStreamingPayload {
    pub success: bool,
    pub start_timepoint: u64,
}

pub const MAGIC_WORD_LEN: usize = 8;
pub type MagicWordBuf = [u8; MAGIC_WORD_LEN];
pub const MAGIC_WORD: MagicWordBuf = *b"CURI BIO";

const TIMESTAMP_LEN: usize = size_of::<u64>();
const CHECKSUM_LEN: usize = size_of::<u32>();
const MIN_PACKET_REMAINDER_SIZE: usize = TIMESTAMP_LEN + CHECKSUM_LEN; // not including packet type in this calculation since it is included in the payload vec
const MAX_PAYLOAD_LEN: usize = 20000 - CHECKSUM_LEN;
#[deku_derive(DekuRead, DekuWrite)]
#[derive(PartialEq)]
#[deku(magic = b"CURI BIO", endian = "little")]
pub struct Packet {
    // PACKET HEADER
    // magic_word: [u8; 8] // this is handled by deku, keeping here for clarity of the packet structure
    #[deku(temp, temp_value = "Self::get_remainder_size(payload.len())")]
    packet_remainder_size: u16,
    // PACKET BASE
    timestamp: u64,
    // packet_type: u8 // this is handled by deku, keeping here for clarity of the packet structure
    // PAYLOAD
    #[deku(
        count = "(*packet_remainder_size as usize) - MIN_PACKET_REMAINDER_SIZE",
        assert = "payload.len() <= MAX_PAYLOAD_LEN - 1"  // subtract one since payload includes the packet type bytes here// TODO test this
    )]
    pub payload: Vec<u8>, // packet type is included in this vec to make working with deku easier
    // CHECKSUM
    #[deku(temp, temp_value = "0")]
    // TODO set temp_value = something that will calculate the CRC
    // TODO validate checksum when reading
    checksum: u32,
}

impl Packet {
    fn get_remainder_size(payload_len: usize) -> u16 {
        (payload_len + MIN_PACKET_REMAINDER_SIZE) as u16
    }

    fn packet_type(&self) -> Result<PacketType> {
        if self.payload.is_empty() {
            bail!("payload vec is completely empty, should at least contain one element for the packet type");
        }
        Ok(self.payload[0])
    }

    fn true_payload(&self) -> Result<&[u8]> {
        if self.payload.is_empty() {
            bail!("payload vec is completely empty, should at least contain one element for the packet type");
        }
        Ok(&self.payload[1..])
    }
}

impl fmt::Debug for Packet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Packet")
            .field("timestamp", &self.timestamp)
            .field(
                "packet_type",
                &self.packet_type().map_err(|_| None::<PacketType>),
            )
            .field("payload", &self.true_payload().map_err(|_| None::<&[u8]>))
            .finish()
    }
}

impl TryFrom<OutgoingPacketPayloads> for Packet {
    type Error = anyhow::Error;
    fn try_from(payload: OutgoingPacketPayloads) -> Result<Self> {
        let payload = payload.to_bytes().with_context(|| {
            format!("failed to convert outgoing payload to bytes {:?}", payload)
        })?;
        Ok(Self {
            timestamp: 0, // TODO set to current time
            payload,
        })
    }
}

#[cfg(test)]
impl TryFrom<IncomingPacketPayloads> for Packet {
    type Error = anyhow::Error;
    fn try_from(payload: IncomingPacketPayloads) -> Result<Self> {
        let payload = payload.to_bytes().with_context(|| {
            format!("failed to convert incoming payload to bytes {:?}", payload)
        })?;
        Ok(Self {
            timestamp: 0, // TODO set to current time
            payload,
        })
    }
}

pub struct DataParseResult {
    pub unread_bytes: Vec<u8>,
    pub packets: Vec<Packet>,
}

pub fn parse_data_stream(buf: Vec<u8>) -> Result<DataParseResult> {
    let mut packets = vec![];

    let mut unparsed = &buf[..];

    loop {
        match Packet::from_bytes((unparsed, 0)) {
            Ok((remainder, packet)) => {
                packets.push(packet);
                unparsed = remainder.0;
            }
            Err(DekuError::Incomplete(_)) => {
                return Ok(DataParseResult {
                    unread_bytes: unparsed.to_vec(),
                    packets,
                });
            }
            Err(e) => {
                return Err(e).with_context(|| "failed_to parse data stream")?;
                // TODO need to either enable logging from deku or handle its errors in a better way
                // TODO implement
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use deku::DekuError;

    const MAGIC_WORD: [u8; 8] = *b"CURI BIO";

    fn prepend_magic_word(packet_bytes: Vec<u8>) -> Vec<u8> {
        MAGIC_WORD
            .into_iter()
            .chain(packet_bytes.into_iter())
            .collect()
    }

    fn get_generic_status_beacon_payload() -> IncomingPacketPayloads {
        IncomingPacketPayloads::StatusBeacon(IncomingStatusBeaconPayload {
            main_status: 100,
            index_of_thread_with_error: 101,
            module_statuses: [
                0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22,
                23,
            ],
        })
    }

    fn get_generic_status_beacon_packet() -> Packet {
        get_generic_status_beacon_payload()
            .try_into()
            .expect("failed to init packet")
    }

    /// Individual packet tests

    #[test]
    fn write_then_parse_status_beacon() {
        let original_payload = get_generic_status_beacon_payload();
        let packet: Packet = original_payload
            .clone()
            .try_into()
            .expect("failed to init packet");
        let res = packet.to_bytes().expect("failed to write packet");
        let expected_bytes = prepend_magic_word(vec![
            39, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 100, 101, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12,
            13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 0, 0, 0, 0,
        ]);
        assert_eq!(res, expected_bytes);

        let res = Packet::from_bytes((&expected_bytes, 0))
            .expect("failed to parse packet")
            .1;
        assert_eq!(res, packet);

        let (remainder, res) =
            IncomingPacketPayloads::from_bytes((&res.payload, 0)).expect("failed to parse payload");
        assert_eq!(remainder.1, 0);
        assert_eq!(res, original_payload);
    }

    #[test]
    fn parse_invalid_payload_type() {
        let test_bytes = prepend_magic_word(vec![13, 0, 0, 0, 0, 0, 0, 0, 0, 0, 250, 0, 0, 0, 0]);
        let (remainder, packet) =
            Packet::from_bytes((&test_bytes, 0)).expect("packet should parse");
        assert_eq!(remainder.1, 0);

        let res = IncomingPacketPayloads::from_bytes((&packet.payload, 0))
            .expect_err("payload shouldn't parse");

        assert!(matches!(res, DekuError::Parse(_)));
        assert!(res.to_string().contains("250"));
    }

    #[test]
    fn packet_exceeding_max_len() {
        let packet_remainder_size = (TIMESTAMP_LEN + MAX_PAYLOAD_LEN + CHECKSUM_LEN) as u16;
        let test_bytes: Vec<u8> = prepend_magic_word(
            packet_remainder_size
                .to_le_bytes()
                .to_vec()
                .into_iter()
                .chain((0..packet_remainder_size).map(|_| 0))
                .collect(),
        );
        // let test_bytes = prepend_magic_word(test_vec);
        let res = Packet::from_bytes((&test_bytes, 0)).expect_err("packet shouldn't parse");
        assert!(matches!(res, DekuError::Assertion(_)));
        assert!(res.to_string().contains("MAX_PAYLOAD_LEN"));
    }

    // TODO test parsing invalid packet type

    #[test]
    fn parse_data_stream__handle_packet_and_unread_bytes() {
        let test_packet = get_generic_status_beacon_packet();
        let test_bytes = test_packet.to_bytes().expect("failed to write packet");
        let test_bytes_len = test_bytes.len();

        let almost_two_packets: Vec<u8> = test_bytes
            .into_iter()
            .cycle()
            .take(test_bytes_len * 2 - 1)
            .collect();
        let res =
            parse_data_stream(almost_two_packets.clone()).expect("failed to parse data stream");

        assert_eq!(res.unread_bytes, almost_two_packets[test_bytes_len..]);
        assert_eq!(res.packets, vec![test_packet]);
    }
}
