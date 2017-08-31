extern crate getopts;
extern crate byteorder;
#[macro_use] extern crate bitflags;

mod diameter;
mod gy;

use getopts::{Options, Matches};
use std::process;
use std::env;
use std::sync::Arc;
use std::io::{Write, Read};
use std::net::{TcpListener, TcpStream, IpAddr, Ipv4Addr};
use std::thread;
use std::str::FromStr;
use std::convert::From;
use diameter::message_builder::MessageBuilder;
use diameter::message_header::MessageHeader;
use diameter::message_flags;
use diameter::result_codes;
use diameter::avps;
use diameter::avp_flags;
use diameter::commands;

struct Config {
    origin_host: String,
    origin_realm: String,
    product_name: String,
    firmware_revision: u32,
    host_ip_address: Ipv4Addr,
    vendor_id: u32,
    validity_time: u32,
    time: u32,
    time_threshold: u32,
    input_octets: u64,
    output_octets: u64,
    total_octets: u64,
    volume_threshold: u32,
}

#[derive(Debug)]
pub enum ClientError {
    IoError(std::io::Error),
    ParseError(diameter::ParseError),
    ReadBufferOverflow(u32),
    DisconnectRequested
}

impl From<std::io::Error> for ClientError {
    fn from(err: std::io::Error) -> Self {
        ClientError::IoError(err)
    }
}

impl From<diameter::ParseError> for ClientError {
    fn from(err: diameter::ParseError) -> Self {
        ClientError::ParseError(err)
    }
}

fn handle_client(mut stream: TcpStream, config: Arc<Config>) {
    let mut ccr_buffer = gy::CcRequest::new();
    let mut write_buffer = vec![0u8; 2048];
    let mut read_buffer = [0u8; 16 * 1024];
    let address = stream.peer_addr().unwrap();
    println!("[{}] Client connected", address);
    loop {
        if let Err(e) = process_client(&config, &mut stream, &mut read_buffer, &mut write_buffer, &mut ccr_buffer) {
            match e {
                ClientError::DisconnectRequested => {
                    println!("[{}] Client gracefully disconnected", address);
                }
                ClientError::IoError(e) => {
                    println!("[{}] I/O Error: {}", address, e);
                }
                ClientError::ReadBufferOverflow(size) => {
                    println!("[{}] Got a too large packet: {}", address, size);
                }
                ClientError::ParseError(e) => {
                    println!("[{}] Packet parsing failed: {}", address, e.description());
                }
            };
            break;
        }
    }
}

fn process_client(config: &Config, stream: &mut TcpStream, read_buffer: &mut [u8], write_buffer: &mut Vec<u8>, ccr_buffer: &mut gy::CcRequest) -> Result<(), ClientError> {
    let header = read_header(stream)?;
    let payload = read_payload(&header, stream, read_buffer)?;
    handle_packet_and_flush(config, &header, payload, write_buffer, ccr_buffer, stream)?;
    Ok(())
}

fn read_header(stream: &mut TcpStream) -> Result<MessageHeader, ClientError> {
    let mut headbuf = [0u8; 20];
    stream.read_exact(&mut headbuf)?; // TODO: Read data in larger batches
    Ok(MessageHeader::parse(&headbuf)?)
}

fn read_payload<'a>(header: &MessageHeader, stream: &mut TcpStream, buffer: &'a mut [u8]) -> Result<&'a [u8], ClientError> {
    let plen = header.payload_len();
    let plen_us = plen as usize;
    if plen_us > buffer.len() {
        return Err(ClientError::ReadBufferOverflow(plen));
    }
    stream.read_exact(&mut buffer[0..plen_us])?;
    Ok(&buffer[0..plen_us])
}

fn handle_packet_and_flush(config: &Config, header: &MessageHeader, payload: &[u8], output: &mut Vec<u8>, ccr: &mut gy::CcRequest, stream: &mut TcpStream) -> Result<(), ClientError> {
    output.clear();
    let connected = handle_packet(&config, &header, payload, output, ccr);
    stream.write_all(&output)?;
    if !connected {
        return Err(ClientError::DisconnectRequested);
    }
    Ok(())
}

fn handle_packet(config: &Config, header: &MessageHeader, payload: &[u8], output: &mut Vec<u8>, ccr: &mut gy::CcRequest) -> bool {
    if header.flags.contains(message_flags::REQUEST) {
        match header.command_id {
            commands::CAPABILITIES_EXCHANGE => handle_cer(config, header, output),
            commands::DEVICE_WATCHDOG => handle_dwr(config, header, output),
            commands::DISCONNECT_PEER => {
                handle_dpr(config, header, output);
                return false;
            }
            gy::commands::CREDIT_CONTROL => handle_gy_ccr(config, header, payload, output, ccr),
            _ => handle_unknown(config, header, output)
        }
    }
    true
}

fn handle_cer(config: &Config, header: &MessageHeader, output: &mut Vec<u8>) {
    MessageBuilder::new(output, message_flags::NONE, header.command_id, header.hop_by_hop, header.end_to_end)
        .put_avp_u32(avps::RESULT_CODE, avp_flags::NONE, result_codes::SUCCESS)
        .put_avp_bytes(avps::ORIGIN_HOST, avp_flags::NONE, config.origin_host.as_bytes())
        .put_avp_bytes(avps::ORIGIN_REALM, avp_flags::NONE, config.origin_realm.as_bytes())
        .put_avp_u32(avps::VENDOR_ID, avp_flags::NONE, config.vendor_id)
        .put_avp_bytes(avps::PRODUCT_NAME, avp_flags::NONE, config.product_name.as_bytes())
        .put_avp_u32(avps::FIRMWARE_REVISION, avp_flags::NONE, config.firmware_revision)
        .put_avp_address(avps::HOST_IP_ADDRESS, avp_flags::NONE, config.host_ip_address)
        .put_avp_u32(avps::SUPPORTED_VENDOR_ID, avp_flags::NONE, gy::TGPP_VENDOR_ID)
        .put_avp_u32(avps::AUTH_APPLICATION_ID, avp_flags::NONE, gy::APPLICATION_ID);
}

fn handle_dwr(config: &Config, header: &MessageHeader, output: &mut Vec<u8>) {
    MessageBuilder::new(output, message_flags::NONE, header.command_id, header.hop_by_hop, header.end_to_end)
        .put_avp_u32(avps::RESULT_CODE, avp_flags::NONE, result_codes::SUCCESS)
        .put_avp_bytes(avps::ORIGIN_HOST, avp_flags::NONE, config.origin_host.as_bytes())
        .put_avp_bytes(avps::ORIGIN_REALM, avp_flags::NONE, config.origin_realm.as_bytes());
}

fn handle_dpr(config: &Config, header: &MessageHeader, output: &mut Vec<u8>) {
    MessageBuilder::new(output, message_flags::NONE, header.command_id, header.hop_by_hop, header.end_to_end)
        .put_avp_u32(avps::RESULT_CODE, avp_flags::NONE, result_codes::SUCCESS)
        .put_avp_bytes(avps::ORIGIN_HOST, avp_flags::NONE, config.origin_host.as_bytes())
        .put_avp_bytes(avps::ORIGIN_REALM, avp_flags::NONE, config.origin_realm.as_bytes());
}

fn handle_gy_ccr(config: &Config, header: &MessageHeader, payload: &[u8], output: &mut Vec<u8>, ccr: &mut gy::CcRequest) {
    let result_code = match ccr.parse(payload) {
        Ok(()) => result_codes::SUCCESS,
        Err(e) => e.result_code(),
    };
    let new_flags = header.flags & message_flags::PROXIABLE;
    let mut mb = MessageBuilder::new(output, new_flags, header.command_id, header.hop_by_hop, header.end_to_end);
    mb.put_avp_bytes_nonempty(avps::SESSION_ID, avp_flags::NONE, &ccr.session_id);
    mb.put_avp_u32(avps::RESULT_CODE, avp_flags::NONE, result_code);
    mb.put_avp_bytes(avps::ORIGIN_HOST, avp_flags::NONE, config.origin_host.as_bytes());
    mb.put_avp_bytes(avps::ORIGIN_REALM, avp_flags::NONE, config.origin_realm.as_bytes());
    mb.put_avp_u32(avps::AUTH_APPLICATION_ID, avp_flags::NONE, gy::APPLICATION_ID);
    mb.put_avp_u32_option(gy::avps::CC_REQUEST_TYPE, avp_flags::NONE, ccr.request_type);
    mb.put_avp_u32_option(gy::avps::CC_REQUEST_NUMBER, avp_flags::NONE, ccr.request_number);
    if result_code == result_codes::SUCCESS {
        mb.put_avp_u32(gy::avps::CC_SESSION_FAILOVER, avp_flags::NONE, 1);
        mb.put_avp_empty(gy::avps::MULTIPLE_SERVICES_INDICATOR, avp_flags::NONE);
        for service in ccr.services.iter() {
            put_service(config, service, &mut mb);
        }
    }
}

fn put_service(config: &Config, service: &gy::CcService, builder: &mut MessageBuilder) {
    let mut sb = builder.begin_avp(gy::avps::MULTIPLE_SERVICES_CC, avp_flags::NONE);
    sb.put_avp_u32(avps::RESULT_CODE, avp_flags::NONE, result_codes::SUCCESS);
    sb.put_avp_u32_option(gy::avps::SERVICE_IDENTIFIER, avp_flags::NONE, service.service_id);
    sb.put_avp_u32_option(gy::avps::RATING_GROUP, avp_flags::NONE, service.rating_group);
    if service.units_requested {
        sb.put_avp_u32_nonzero(gy::avps::VALIDITY_TIME, avp_flags::NONE, config.validity_time);
        sb.put_avp_u32_nonzero(gy::avps::TIME_QUOTA_THRESHOLD, avp_flags::NONE, config.time_threshold);
        sb.put_avp_u32_nonzero(gy::avps::VOLUME_QUOTA_THRESHOLD, avp_flags::NONE, config.volume_threshold);
        sb.begin_avp(gy::avps::GRANTED_SERVICE_UNIT, avp_flags::NONE)
            .put_avp_u32_nonzero(gy::avps::CC_TIME, avp_flags::NONE, config.time)
            .put_avp_u64_nonzero(gy::avps::CC_INPUT_OCTETS, avp_flags::NONE, config.input_octets)
            .put_avp_u64_nonzero(gy::avps::CC_OUTPUT_OCTETS, avp_flags::NONE, config.output_octets)
            .put_avp_u64_nonzero(gy::avps::CC_TOTAL_OCTETS, avp_flags::NONE, config.total_octets);
    }
}

fn handle_unknown(config: &Config, header: &MessageHeader, output: &mut Vec<u8>) {
    let result_code = match header.command_id.application_id {
        diameter::BASE_APPLICATION_ID => result_codes::COMMAND_UNSUPPORTED,
        gy::APPLICATION_ID => result_codes::COMMAND_UNSUPPORTED,
        _ => result_codes::APPLICATION_UNSUPPORTED
    };
    MessageBuilder::new(output, message_flags::ERROR, header.command_id, header.hop_by_hop, header.end_to_end)
        .put_avp_u32(avps::RESULT_CODE, avp_flags::NONE, result_code)
        .put_avp_bytes(avps::ORIGIN_HOST, avp_flags::NONE, config.origin_host.as_bytes())
        .put_avp_bytes(avps::ORIGIN_REALM, avp_flags::NONE, config.origin_realm.as_bytes());
}

fn parse_args() -> Matches {
    let args: Vec<String> = env::args().collect();
    let program = &args[0];
    let mut opts = Options::new();
    opts.optflag("h", "help", "Show this usage message.");
    opts.optopt("p", "listen-port", "Port to listen on.", "PORT");
    opts.optopt("l", "listen-address", "Address to listen on.", "ADDRESS");
    opts.optopt("", "origin-host", "Value for the Origin-Host AVP.", "STRING");
    opts.optopt("", "origin-realm", "Value for the Origin-Realm AVP.", "STRING");
    opts.optopt("", "product-name", "Value for the Product-Name AVP.", "STRING");
    opts.optopt("", "firmware-revision", "Value for the Firmware-Revision AVP.", "NUMBER");
    opts.optopt("", "host-ip-address", "Value for the Host-Ip-Address AVP.", "STRING");
    opts.optopt("", "vendor-id", "Value for the Vendor-ID AVP.", "NUMBER");
    opts.optopt("", "validity-time", "Value for the Validity-Time AVP.", "SECONDS");
    opts.optopt("", "time", "Value for the CC-Time AVP.", "SECONDS");
    opts.optopt("", "time-threshold", "Value for the Time-Threshold AVP.", "SECONDS");
    opts.optopt("", "input-octets", "Value for the CC-Input-Octets AVP.", "BYTES");
    opts.optopt("", "output-octets", "Value for the CC-Output-Octets AVP.", "BYTES");
    opts.optopt("", "total-octets", "Value for the CC-Total-Octets AVP.", "BYTES");
    opts.optopt("", "volume-threshold", "Value for the Volume-Threshold AVP.", "BYTES");

    let matches = match opts.parse(&args[1..]) {
        Ok(m) => { m }
        Err(e) => {
            println!("{}", e.to_string());
            process::exit(1);
        }
    };
    if matches.opt_present("h") {
        println!("{}", opts.usage(&format!("Usage: {} [options]", program)));
        process::exit(0);
    }
    matches
}

fn parse_config(matches: &Matches) -> Config {
    let host_ip_address = get_str(matches, "host-ip-address", "127.0.0.1");
    if Ipv4Addr::from_str(&host_ip_address).is_err() {
        println!("Failed to parse host_ip_address: {}", host_ip_address);
        process::exit(1);
    }

    Config {
        origin_host: get_str(matches, "origin-host", "dummy_host"),
        origin_realm: get_str(matches, "origin-realm", "dummy_realm"),
        product_name: get_str(matches, "product-name", "Dummy OCS"),
        firmware_revision: get_u32(matches, "firmware-revision", 1),
        host_ip_address: Ipv4Addr::from_str(&*host_ip_address).unwrap(),
        vendor_id: get_u32(matches, "vendor-id", 0xFFFFFFFF),
        validity_time: get_u32(matches, "validity-time", 15 * 60),
        time: get_u32(matches, "time", 0),
        time_threshold: get_u32(matches, "time-threshold", 0),
        input_octets: get_u64(matches, "input-octets", 0),
        output_octets: get_u64(matches, "output-octets", 0),
        total_octets: get_u64(matches, "total-octets", 1024 * 1024),
        volume_threshold: get_u32(matches, "volume-threshold", 0)
    }
}

fn get_str(matches: &Matches, key: &str, def: &str) -> String {
    matches.opt_str(key).unwrap_or(def.to_string())
}

fn get_u32(matches: &Matches, key: &str, def: u32) -> u32 {
    matches.opt_str(key).map_or(def, |x| x.parse().unwrap())
}

fn get_u64(matches: &Matches, key: &str, def: u64) -> u64 {
    matches.opt_str(key).map_or(def, |x| x.parse().unwrap())
}

fn main() {
    let opt_matches = parse_args();
    let port = opt_matches.opt_str("p").map_or(3868, |x| x.parse::<u16>().unwrap());
    let address = IpAddr::from_str(&*get_str(&opt_matches, "l", "127.0.0.1")).unwrap();
    let config = Arc::new(parse_config(&opt_matches));

    let listener = TcpListener::bind((address, port)).unwrap();
    println!("Listening to {}:{}", address, port);

    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                let config = config.clone();
                thread::spawn(move || handle_client(stream, config));
            }
            Err(e) => {
                println!("Accept failed: {}", e.to_string());
            }
        }
    }
}
