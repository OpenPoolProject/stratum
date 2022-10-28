use crate::{Error, Result};
use extended_primitives::Buffer;
use serde::{Deserialize, Serialize};
use std::{fmt, sync::Arc};
use tokio::sync::Mutex;

#[derive(Debug)]
pub struct VarDiffBuffer {
    pub(crate) pos: usize,
    pub(crate) used: usize,
    pub(crate) data: [i64; 90],
}

impl VarDiffBuffer {
    pub fn new() -> VarDiffBuffer {
        VarDiffBuffer {
            pos: 0,
            used: 0,
            data: [0; 90],
        }
    }

    pub(crate) fn append(&mut self, time: i64) {
        self.data[self.pos] = time;
        self.pos += 1;
        self.pos %= 90;

        if self.used < 90 {
            self.used += 1;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.pos = 0;
        self.used = 0;
    }

    pub(crate) fn avg(&self) -> f64 {
        let mut count = 90;

        if self.used < 90 {
            count = self.pos;
        }

        let mut total: i64 = 0;
        for i in 0..count {
            total += self.data[i]
        }

        (total as f64) / (count as f64)
    }
}

pub const EX_MAGIC_NUMBER: u8 = 0x7F;

#[derive(Clone, Default)]
pub struct ReadyIndicator(Arc<Mutex<bool>>);

impl ReadyIndicator {
    pub fn new(ready: bool) -> Self {
        ReadyIndicator(Arc::new(Mutex::new(ready)))
    }

    pub async fn ready(&self) {
        *self.0.lock().await = true;
    }

    pub async fn not_ready(&self) {
        *self.0.lock().await = false;
    }

    pub async fn inner(&self) -> bool {
        *self.0.lock().await
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ID {
    Num(u64),
    Str(String),
    Null(serde_json::Value),
}

impl ID {
    pub fn null() -> ID {
        ID::Null(serde_json::Value::Null)
    }
}

impl std::fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ID::Num(ref e) => write!(f, "{e}"),
            ID::Str(ref e) => write!(f, "{e}"),
            ID::Null(ref _e) => write!(f, "null"),
        }
    }
}

#[derive(Clone, Debug)]
pub enum MessageValue {
    StratumV1(serde_json::map::Map<String, serde_json::Value>),
    ExMessage(ExMessageGeneric),
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum MessageTypes {
    RegisterWorker,
    SubmitShare,
    SubmitShareWithTime,
    SubmitShareWithVersion,
    SubmitShareWithTimeAndVersion,
    UnregisterWorker,
    MiningSetDiff,

    Unknown(u8),
}

impl MessageTypes {
    pub fn from_u8(cmd: u8) -> Self {
        match cmd {
            0x01 => MessageTypes::RegisterWorker,
            0x02 => MessageTypes::SubmitShare,
            0x03 => MessageTypes::SubmitShareWithTime,
            0x04 => MessageTypes::UnregisterWorker,
            0x05 => MessageTypes::MiningSetDiff,
            //@note not sure why these are so far after the originals. Makes me think there are
            //other messages we are missing here, but can figure that out later.
            0x12 => MessageTypes::SubmitShareWithVersion,
            0x13 => MessageTypes::SubmitShareWithTimeAndVersion,
            _ => MessageTypes::Unknown(cmd),
        }
    }
}

impl fmt::Display for MessageTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            MessageTypes::RegisterWorker => write!(f, "exMessageRegisterWorker"),
            MessageTypes::SubmitShare => write!(f, "exMessageSubmitShare"),
            MessageTypes::SubmitShareWithTime => write!(f, "exMessageSubmitShare"),
            MessageTypes::UnregisterWorker => write!(f, "exMessageUnregisterWorker"),
            MessageTypes::MiningSetDiff => write!(f, "exMessageMiningSetDiff"),
            MessageTypes::SubmitShareWithVersion => write!(f, "exMessageSubmitShare"),
            MessageTypes::SubmitShareWithTimeAndVersion => write!(f, "exMessageSubmitShare"),
            _ => write!(f, ""),
        }
    }
}

// ex-message: BTC Agent Messages
//   magic_number	uint8_t		magic number for Ex-Message, always 0x7F
//   type/cmd		uint8_t		message type
//   length			uint16_t	message length (include header self)
//   message_body	uint8_t[]	message body
#[derive(Clone, Debug)]
pub struct ExMessageGeneric {
    pub magic_number: u8,
    pub cmd: MessageTypes,
    pub length: u16,
    pub body: Buffer,
}

impl ExMessageGeneric {
    pub fn from_buffer(buffer: &mut Buffer) -> Result<Self> {
        let magic_number = buffer.read_u8().map_err(|_| Error::BrokenExHeader)?;
        let cmd = buffer.read_u8().map_err(|_| Error::BrokenExHeader)?;
        let length = buffer.read_u16().map_err(|_| Error::BrokenExHeader)?;

        let body = buffer.clone();

        let cmd = MessageTypes::from_u8(cmd);

        if length as usize != body.len() {
            return Err(Error::BrokenExHeader);
        }

        if let MessageTypes::Unknown(_) = cmd {
            return Err(Error::BrokenExHeader);
        }

        Ok(ExMessageGeneric {
            magic_number,
            cmd,
            length,
            body,
        })
    }
}

#[derive(Clone, Debug)]
pub struct GlobalVars {
    pub server_id: u8,
}

impl GlobalVars {
    pub fn new(server_id: u8) -> Self {
        GlobalVars { server_id }
    }
}
