use byteorder::{ByteOrder, BigEndian};
use super::ParseError;
use super::commands::CommandId;
use super::message_flags::MessageFlags;

pub const MESSAGE_HEADER_SIZE: u32 = 20;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct HopByHop(pub u32);

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct EndToEnd(pub u32);

pub struct MessageHeader {
    pub command_id: CommandId,
    pub flags: MessageFlags,
    pub hop_by_hop: HopByHop,
    pub end_to_end: EndToEnd,
    pub length: u32,
}

impl MessageHeader {
    pub fn parse(buffer: &[u8; MESSAGE_HEADER_SIZE as usize]) -> Result<MessageHeader, ParseError> {
        let length = BigEndian::read_u32(&buffer[0..4]) & 0x00FFFFFF;
        if length < MESSAGE_HEADER_SIZE {
            return Err(ParseError::InvalidMessageLength);
        }
        let flags_and_code = BigEndian::read_u32(&buffer[4..8]);
        let flags = try!(MessageFlags::from_bits((flags_and_code >> 24) as u8).ok_or(ParseError::InvalidBitInHeader));
        Ok(MessageHeader {
            command_id: CommandId {
                code: flags_and_code & 0x00FFFFFF,
                application_id: BigEndian::read_u32(&buffer[8..12]),
            },
            length: length,
            flags: flags,
            hop_by_hop: HopByHop(BigEndian::read_u32(&buffer[12..16])),
            end_to_end: EndToEnd(BigEndian::read_u32(&buffer[16..20]))
        })
    }

    #[allow(dead_code)]
    pub fn total_len(&self) -> u32 {
        self.length
    }

    pub fn payload_len(&self) -> u32 {
        self.length - MESSAGE_HEADER_SIZE
    }
}
