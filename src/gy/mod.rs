use diameter;
use diameter::ParseError;
use diameter::avps::AvpId;
use diameter::avp_parsers::{parse_avps, parse_u32};

pub const TGPP_VENDOR_ID: u32 = 10415;
pub const APPLICATION_ID: u32 = 4;

pub mod commands {
    use diameter::commands::CommandId;
    use super::APPLICATION_ID;

    pub const CREDIT_CONTROL: CommandId = CommandId { code: 272, application_id: APPLICATION_ID };
}

pub mod avps {
    use diameter::avps::AvpId;

    pub const CC_REQUEST_NUMBER: AvpId = AvpId { code: 415, vendor_id: 0 };
    pub const CC_REQUEST_TYPE: AvpId = AvpId { code: 416, vendor_id: 0 };
    pub const CC_SESSION_FAILOVER: AvpId = AvpId { code: 418, vendor_id: 0 };
    pub const MULTIPLE_SERVICES_INDICATOR: AvpId = AvpId { code: 455, vendor_id: 0 };
    pub const MULTIPLE_SERVICES_CC: AvpId = AvpId { code: 456, vendor_id: 0 };
    pub const REQUESTED_SERVICE_UNIT: AvpId = AvpId { code: 437, vendor_id: 0 };
    pub const SERVICE_IDENTIFIER: AvpId = AvpId { code: 439, vendor_id: 0 };
    pub const RATING_GROUP: AvpId = AvpId { code: 432, vendor_id: 0 };
    pub const GRANTED_SERVICE_UNIT: AvpId = AvpId { code: 431, vendor_id: 0 };
    pub const VALIDITY_TIME: AvpId = AvpId { code: 448, vendor_id: 0 };
    pub const CC_INPUT_OCTETS: AvpId = AvpId { code: 412, vendor_id: 0 };
    pub const CC_OUTPUT_OCTETS: AvpId = AvpId { code: 414, vendor_id: 0 };
    pub const CC_TOTAL_OCTETS: AvpId = AvpId { code: 421, vendor_id: 0 };
    pub const CC_TIME: AvpId = AvpId { code: 420, vendor_id: 0 };
}

pub struct CcRequest {
    pub session_id: Vec<u8>,
    pub service_context_id: Vec<u8>,
    pub request_type: Option<u32>,
    pub request_number: Option<u32>,
    pub services: Vec<CcService>,
}

pub struct CcService {
    pub service_id: Option<u32>,
    pub rating_group: Option<u32>,
    pub units_requested: bool
}

impl CcRequest {
    pub fn new() -> Self {
        CcRequest {
            request_type: None, request_number: None, session_id: vec![],
            services: vec![], service_context_id: vec![]
        }
    }

    pub fn parse(&mut self, buffer: &[u8]) -> Result<(), ParseError> {
        self.session_id.clear();
        self.service_context_id.clear();
        self.request_type = None;
        self.request_number = None;
        self.services.clear();
        parse_avps(buffer, &parse_ccr_avp, self)
    }
}

fn parse_ccr_avp(avp_id: AvpId, payload: &[u8], result: &mut CcRequest) -> Result<(), ParseError> {
    match avp_id {
        diameter::avps::SESSION_ID => {
            ok_or(result.session_id.is_empty(), ParseError::AvpOccursTooManyTimes)?;
            ok_or(!payload.is_empty(), ParseError::InvalidAvpValue)?;
            result.session_id.extend_from_slice(payload);
        }
        avps::CC_REQUEST_NUMBER => {
            ok_or(result.request_number.is_none(), ParseError::AvpOccursTooManyTimes)?;
            result.request_number = Some(parse_u32(payload)?);
        }
        avps::CC_REQUEST_TYPE => {
            ok_or(result.request_type.is_none(), ParseError::AvpOccursTooManyTimes)?;
            result.request_type = Some(parse_u32(payload)?);
        }
        avps::MULTIPLE_SERVICES_CC => {
            result.services.push(parse_service(payload)?);
        }
        _ => {}
    }
    Ok(())
}

fn parse_service(buffer: &[u8]) -> Result<CcService, ParseError> {
    let mut service = CcService { service_id: None, rating_group: None, units_requested: false };
    parse_avps(buffer, &parse_service_avp, &mut service)?;
    Ok(service)
}

fn parse_service_avp(avp_key: AvpId, payload: &[u8], result: &mut CcService) -> Result<(), ParseError> {
    match avp_key {
        avps::SERVICE_IDENTIFIER => {
            ok_or(result.service_id.is_none(), ParseError::AvpOccursTooManyTimes)?;
            result.service_id = Some(parse_u32(payload)?);
        }
        avps::RATING_GROUP => {
            ok_or(result.rating_group.is_none(), ParseError::AvpOccursTooManyTimes)?;
            result.rating_group = Some(parse_u32(payload)?);
        }
        avps::REQUESTED_SERVICE_UNIT => {
            result.units_requested = true;
        }
        _ => {}
    }
    Ok(())
}

#[inline]
fn ok_or<E>(v: bool, err: E) -> Result<(), E> {
    if v {
        Ok(())
    } else {
        Err(err)
    }
}
