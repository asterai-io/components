use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Copy, PartialEq)]
#[serde(from = "u8", into = "u8")]
pub enum GatewayOpcode {
    Dispatch,
    Heartbeat,
    Identify,
    Resume,
    Reconnect,
    InvalidSession,
    Hello,
    HeartbeatAck,
    Unknown(u8),
}

impl From<u8> for GatewayOpcode {
    fn from(value: u8) -> Self {
        match value {
            0 => Self::Dispatch,
            1 => Self::Heartbeat,
            2 => Self::Identify,
            6 => Self::Resume,
            7 => Self::Reconnect,
            9 => Self::InvalidSession,
            10 => Self::Hello,
            11 => Self::HeartbeatAck,
            other => Self::Unknown(other),
        }
    }
}

impl From<GatewayOpcode> for u8 {
    fn from(op: GatewayOpcode) -> u8 {
        match op {
            GatewayOpcode::Dispatch => 0,
            GatewayOpcode::Heartbeat => 1,
            GatewayOpcode::Identify => 2,
            GatewayOpcode::Resume => 6,
            GatewayOpcode::Reconnect => 7,
            GatewayOpcode::InvalidSession => 9,
            GatewayOpcode::Hello => 10,
            GatewayOpcode::HeartbeatAck => 11,
            GatewayOpcode::Unknown(v) => v,
        }
    }
}
