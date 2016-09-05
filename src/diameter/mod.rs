
pub const BASE_APPLICATION_ID: u32 = 0;

pub mod commands {
    use super::BASE_APPLICATION_ID;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct CommandId {
        pub code: u32,
        pub application_id: u32,
    }

    pub const CAPABILITIES_EXCHANGE: CommandId = CommandId { code: 257, application_id: BASE_APPLICATION_ID };
    pub const DEVICE_WATCHDOG: CommandId = CommandId { code: 280, application_id: BASE_APPLICATION_ID };
    pub const DISCONNECT_PEER: CommandId = CommandId { code: 282, application_id: BASE_APPLICATION_ID };
}

pub mod avps {
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub struct AvpId {
        pub code: u32,
        pub vendor_id: u32
    }

    macro_rules! define_constants {
        (
            $typename:ident $h1:ident, $h2:ident ;
            $($name:ident $v1:expr , $v2:expr ;)*
        ) => (
            $( pub const $name : $typename = $typename { $h1: $v1, $h2: $v2 }; )*
        )
    }

    define_constants!(
        AvpId                    code, vendor_id;
        SESSION_ID                263,         0;
        RESULT_CODE               268,         0;
        ORIGIN_HOST               264,         0;
        ORIGIN_REALM              296,         0;
        VENDOR_ID                 266,         0;
        PRODUCT_NAME              269,         0;
        FIRMWARE_REVISION         267,         0;
        SUPPORTED_VENDOR_ID       265,         0;
        AUTH_APPLICATION_ID       258,         0;
    );
}

pub mod avp_flags {
    bitflags! {
        pub flags AvpFlags: u8 {
            const VENDOR    = 0x80,
            const MANDATORY = 0x40,
            const PROTECTED = 0x20,
        }
    }

    pub const NONE: AvpFlags = AvpFlags { bits: 0 };
}

pub mod message_flags {
    bitflags! {
        pub flags MessageFlags: u8 {
            const REQUEST       = 0x80,
            const PROXIABLE     = 0x40,
            const ERROR         = 0x20,
            const RETRANSMITTED = 0x10,
        }
    }

    pub const NONE: MessageFlags = MessageFlags { bits: 0 };
}

pub mod result_codes {
    pub const SUCCESS: u32 = 2001;
    pub const COMMAND_UNSUPPORTED: u32 = 3001;
    pub const APPLICATION_UNSUPPORTED: u32 = 3007;
}

pub mod avp_header;
pub mod avp_parsers;
pub mod message_builder;
pub mod message_header;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ParseError {
    InvalidMessageLength,
    InvalidBitInHeader,
    InvalidAvpLength,
    InvalidAvpValue,
    InvalidAvpBits,
    AvpOccursTooManyTimes,
}

impl ParseError {
    pub fn description(&self) -> &str {
        match *self {
            ParseError::InvalidMessageLength => "invalid message length",
            ParseError::InvalidBitInHeader => "invalid bit in message header",
            ParseError::InvalidAvpLength => "invalid AVP length",
            ParseError::InvalidAvpValue => "invalid AVP value",
            ParseError::InvalidAvpBits => "invalid bits in AVP header",
            ParseError::AvpOccursTooManyTimes => "AVP occurs too many times",
        }
    }

    pub fn result_code(&self) -> u32 {
        match *self {
            ParseError::InvalidMessageLength => 5015,
            ParseError::InvalidBitInHeader => 5013,
            ParseError::InvalidAvpLength => 5014,
            ParseError::InvalidAvpValue => 5004,
            ParseError::InvalidAvpBits => 3009, // TODO: Error bit should be set in replies
            ParseError::AvpOccursTooManyTimes => 5009
        }
    }
}
