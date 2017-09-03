extern crate byteorder;

use byteorder::{ByteOrder, BigEndian};
use std::net::IpAddr;
use super::avps::AvpId;
use super::avp_flags::AvpFlags;
use super::commands::CommandId;
use super::message_flags::MessageFlags;
use super::message_header::{EndToEnd, HopByHop};

pub struct MessageBuilder<'a> {
    buffer: &'a mut Vec<u8>,
    start_pos: usize,
    is_message: bool,
}

const PROTOCOL_VERSION: u32 = 0x01000000;
const NORMAL_HEADER_SIZE: u32 = 8;
const VENDOR_HEADER_SIZE: u32 = 12;

impl<'a> MessageBuilder<'a> {
    pub fn new(buffer: &'a mut Vec<u8>, flags: MessageFlags, cmd: CommandId, hop_by_hop: HopByHop, end_to_end: EndToEnd) -> Self {
        let start_pos = buffer.len();
        extend(buffer, 20);
        write_u32(buffer, start_pos, 0); // Temporary value
        write_u32(buffer, start_pos + 4, create_flags_and_code(flags.bits(), cmd.code));
        write_u32(buffer, start_pos + 8, cmd.application_id);
        write_u32(buffer, start_pos + 12, hop_by_hop.0);
        write_u32(buffer, start_pos + 16, end_to_end.0);
        MessageBuilder { buffer: buffer, start_pos: start_pos, is_message: true }
    }

    pub fn put_avp_empty<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags) -> &'b mut MessageBuilder<'a> {
        self.write_header(avp_id, flags, 0);
        self
    }

    pub fn put_avp_u32<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: u32) -> &'b mut MessageBuilder<'a> {
        self.write_header(avp_id, flags, 4);
        let pos = self.buffer.len();
        extend(self.buffer, 4);
        write_u32(self.buffer, pos, value);
        self
    }

    pub fn put_avp_u32_option<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: Option<u32>) -> &'b mut MessageBuilder<'a> {
        if let Some(v) = value {
            self.put_avp_u32(avp_id, flags, v);
        }
        self
    }

    pub fn put_avp_u32_nonzero<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: u32) -> &'b mut MessageBuilder<'a> {
        if value > 0 {
            self.put_avp_u32(avp_id, flags, value);
        }
        self
    }

    pub fn put_avp_u64<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: u64) -> &'b mut MessageBuilder<'a> {
        self.write_header(avp_id, flags, 8);
        let pos = self.buffer.len();
        extend(self.buffer, 8);
        write_u64(self.buffer, pos, value);
        self
    }

    pub fn put_avp_u64_nonzero<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: u64) -> &'b mut MessageBuilder<'a> {
        if value > 0 {
            self.put_avp_u64(avp_id, flags, value);
        }
        self
    }

    pub fn put_avp_bytes<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: &[u8]) -> &'b mut MessageBuilder<'a> {
        self.write_header(avp_id, flags, value.len() as u32);
        self.buffer.extend_from_slice(value);
        self.write_padding();
        self
    }

    pub fn put_avp_bytes_nonempty<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, value: &[u8]) -> &'b mut MessageBuilder<'a> {
        if !value.is_empty() {
            self.put_avp_bytes(avp_id, flags, value);
        }
        self
    }

    pub fn put_avp_address<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags, address: IpAddr) -> &'b mut MessageBuilder<'a> {
        match address {
            IpAddr::V4(a) => {
                self.write_header(avp_id, flags, 2 + 4);
                let pos = self.buffer.len();
                extend(self.buffer, 2);
                write_u16(self.buffer, pos, 1); // https://www.iana.org/assignments/address-family-numbers/address-family-numbers.xhtml
                self.buffer.extend_from_slice(&a.octets());
            }
            IpAddr::V6(a) => {
                self.write_header(avp_id, flags, 2 + 16);
                let pos = self.buffer.len();
                extend(self.buffer, 2);
                write_u16(self.buffer, pos, 2);
                self.buffer.extend_from_slice(&a.octets());
            }
        };
        self.write_padding();
        self
    }

    pub fn begin_avp<'b>(&'b mut self, avp_id: AvpId, flags: AvpFlags) -> MessageBuilder<'b> {
        let start_pos = self.buffer.len();
        self.write_header(avp_id, flags, 0);
        MessageBuilder { buffer: self.buffer, start_pos: start_pos, is_message: false }
    }

    fn write_header(&mut self, avp_id: AvpId, flags: AvpFlags, payload_length: u32) {
        let pos = self.buffer.len();
        if avp_id.vendor_id != 0 {
            extend(self.buffer, VENDOR_HEADER_SIZE as usize);
            write_u32(self.buffer, pos, avp_id.code);
            write_u32(self.buffer, pos + 4, create_flags_and_length(flags.bits(), payload_length + VENDOR_HEADER_SIZE));
            write_u32(self.buffer, pos + 8, avp_id.vendor_id);
        } else {
            extend(self.buffer, NORMAL_HEADER_SIZE as usize);
            write_u32(self.buffer, pos, avp_id.code);
            write_u32(self.buffer, pos + 4, create_flags_and_length(flags.bits(), payload_length + NORMAL_HEADER_SIZE));
        }
    }

    fn write_padding(&mut self) {
        while (self.buffer.len() & 0x03) != 0 {
            self.buffer.push(0);
        }
    }
}

impl<'a> Drop for MessageBuilder<'a> {
    fn drop(&mut self) {
        let len = self.buffer.len() - self.start_pos;
        if self.is_message {
            write_u32(self.buffer, self.start_pos, PROTOCOL_VERSION | len as u32);
        }
        else {
            let pos = self.start_pos + 4;
            let new_value = create_flags_and_length(self.buffer[pos], len as u32);
            write_u32(self.buffer, pos, new_value);
            self.write_padding();
        }
    }
}

fn extend(vec: &mut Vec<u8>, n: usize) {
    let new_size = vec.len() + n;
    vec.resize(new_size, 0);
}

#[inline]
fn write_u16(dst: &mut Vec<u8>, pos: usize, value: u16) {
    BigEndian::write_u16(&mut dst[pos..pos + 2], value);
}

#[inline]
fn write_u32(dst: &mut Vec<u8>, pos: usize, value: u32) {
    BigEndian::write_u32(&mut dst[pos..pos + 4], value);
}

#[inline]
fn write_u64(dst: &mut Vec<u8>, pos: usize, value: u64) {
    BigEndian::write_u64(&mut dst[pos..pos + 8], value);
}

fn create_flags_and_code(flags: u8, code: u32) -> u32 {
    (flags as u32) << 24 | (code & 0x00FFFFFF)
}

fn create_flags_and_length(flags: u8, length: u32) -> u32 {
    (flags as u32 & 0xE0) << 24 | (length & 0x00FFFFFF)
}

#[test]
pub fn testme() {
    let mut bb = vec![0u8; 0];
    {
        let mut mb = MessageBuilder::new(&mut bb, super::message_flags::NONE, super::commands::CAPABILITIES_EXCHANGE, HopByHop(0), EndToEnd(0));
        mb.put_avp_u32(super::avps::ORIGIN_HOST, super::avp_flags::NONE, 50);
    }
    assert_eq!(20 + 12, bb.len());
}

#[test]
pub fn testme2() {
    let mut bb = vec![0u8; 0];
    MessageBuilder::new(&mut bb, super::message_flags::NONE, super::commands::CAPABILITIES_EXCHANGE, HopByHop(0), EndToEnd(0))
        .put_avp_u32(super::avps::ORIGIN_HOST, super::avp_flags::NONE, 50)
        .put_avp_u32(super::avps::ORIGIN_HOST, super::avp_flags::NONE, 50);
    assert_eq!(20 + 2*12, bb.len());
}
