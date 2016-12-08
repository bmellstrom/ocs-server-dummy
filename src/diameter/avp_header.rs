use std::result::Result;
use byteorder::{ByteOrder, BigEndian};
use super::ParseError;
use super::avps::AvpId;
use super::avp_flags;
use super::avp_flags::AvpFlags;

const NORMAL_AVP_HEADER_SIZE: usize = 8;
const VENDOR_AVP_HEADER_SIZE: usize = 12;

#[derive(Debug, Copy, Clone)]
pub struct AvpHeader {
    pub avp_id: AvpId,
    pub flags: AvpFlags,
    pub length: u32,
}

impl AvpHeader {
    pub fn parse(buffer: &[u8]) -> Result<AvpHeader, ParseError> {
        if buffer.len() < NORMAL_AVP_HEADER_SIZE {
            return Err(ParseError::InvalidAvpLength);
        }
        let code = BigEndian::read_u32(&buffer[0..4]);
        let flags_and_length = BigEndian::read_u32(&buffer[4..8]);
        let flags = AvpFlags::from_bits((flags_and_length >> 24) as u8).ok_or(ParseError::InvalidAvpBits)?;
        let vendor_id = if flags.contains(avp_flags::VENDOR) {
            if buffer.len() < VENDOR_AVP_HEADER_SIZE {
                return Err(ParseError::InvalidAvpLength);
            }
            BigEndian::read_u32(&buffer[8..12])
        } else {
            0
        };
        let length = flags_and_length & 0x00FFFFFF;
        Ok(AvpHeader { avp_id: AvpId { code: code, vendor_id: vendor_id }, flags: flags, length: length })
    }

    pub fn header_len(&self) -> usize {
        if self.flags.contains(avp_flags::VENDOR) {
            VENDOR_AVP_HEADER_SIZE
        } else {
            NORMAL_AVP_HEADER_SIZE
        }
    }

    pub fn total_len(&self) -> usize {
        self.length as usize
    }

    #[allow(dead_code)]
    pub fn payload_len(&self) -> usize {
        self.length as usize - self.header_len()
    }
}
