use std::result::Result;
use byteorder::{ByteOrder, BigEndian};
use super::ParseError;
use super::avps::AvpId;
use super::avp_header::AvpHeader;

pub type ParserFn<T> = Fn(AvpId, &[u8], &mut T) -> Result<(), ParseError>;

pub fn parse_avps<T>(buffer: &[u8], avp_parser: &ParserFn<T>, result: &mut T) -> Result<(), ParseError> {
    let mut pos = 0;
    while pos < buffer.len() {
        let header = AvpHeader::parse(&buffer[pos..])?;
        let padded_len = round_up(header.total_len());
        if padded_len > buffer.len() - pos {
            return Err(ParseError::InvalidAvpLength);
        }
        let start = pos + header.header_len();
        let end = pos + header.total_len();
        avp_parser(header.avp_id, &buffer[start..end], result)?;
        pos += padded_len;
    }
    Ok(())
}

fn round_up(value: usize) -> usize {
    (value + 3) & 0xFFFFFFFFFFFFFFFC
}

pub fn parse_u32(buffer: &[u8]) -> Result<u32, ParseError> {
    if buffer.len() != 4 {
        return Err(ParseError::InvalidAvpLength);
    }
    Ok(BigEndian::read_u32(buffer))
}
