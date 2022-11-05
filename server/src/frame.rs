use crate::ID;
#[cfg(feature = "v1")]
use serde::Deserialize;

pub enum Frame {
    // #[cfg(feature = "btcagent")]
    // ExMessage(ExMessage),

    // #[cfg(feature = "v1")]
    // V1(serde_json::map::Map<String, serde_json::Value>),
    #[cfg(feature = "v1")]
    V1(Request),
}

impl Frame {
    pub(crate) fn method(&self) -> &str {
        match self {
            #[cfg(feature = "v1")]
            Frame::V1(req) => &req.method,
        }
    }
}

#[derive(Clone, Deserialize, Debug)]
pub struct Request {
    pub id: ID, // can be number
    pub method: String,
    pub params: serde_json::Value,
}

//@todo move all these to seperate crate, but for now it works.

// ex-message: BTC Agent Messages
//   magic_number	uint8_t		magic number for Ex-Message, always 0x7F
//   type/cmd		uint8_t		message type
//   length			uint16_t	message length (include header self)
//   message_body	uint8_t[]	message body
// #[derive(Clone, Debug)]
// pub struct ExMessageGeneric {
//     pub magic_number: u8,
//     pub cmd: MessageTypes,
//     pub length: u16,
//     pub body: Buffer,
// }
//
// impl ExMessageGeneric {
//     //@todo maybe switch this to bytes.
//     pub fn from_buffer(buffer: &mut Buffer) -> Result<Self> {
//         let magic_number = buffer.read_u8().map_err(|_| Error::BrokenExHeader)?;
//         let cmd = buffer.read_u8().map_err(|_| Error::BrokenExHeader)?;
//         let length = buffer.read_u16().map_err(|_| Error::BrokenExHeader)?;
//
//         let body = buffer.clone();
//
//         let cmd = MessageTypes::from_u8(cmd);
//
//         if length as usize != body.len() {
//             return Err(Error::BrokenExHeader);
//         }
//
//         if let MessageTypes::Unknown(_) = cmd {
//             return Err(Error::BrokenExHeader);
//         }
//
//         Ok(ExMessageGeneric {
//             magic_number,
//             cmd,
//             length,
//             body,
//         })
//     }
// }

// #[derive(Clone, Eq, PartialEq, Debug)]
// pub enum MessageTypes {
//     RegisterWorker,
//     SubmitShare,
//     SubmitShareWithTime,
//     SubmitShareWithVersion,
//     SubmitShareWithTimeAndVersion,
//     UnregisterWorker,
//     MiningSetDiff,
//
//     Unknown(u8),
// }
//
// impl MessageTypes {
//     pub fn from_u8(cmd: u8) -> Self {
//         match cmd {
//             0x01 => MessageTypes::RegisterWorker,
//             0x02 => MessageTypes::SubmitShare,
//             0x03 => MessageTypes::SubmitShareWithTime,
//             0x04 => MessageTypes::UnregisterWorker,
//             0x05 => MessageTypes::MiningSetDiff,
//             //@note not sure why these are so far after the originals. Makes me think there are
//             //other messages we are missing here, but can figure that out later.
//             0x12 => MessageTypes::SubmitShareWithVersion,
//             0x13 => MessageTypes::SubmitShareWithTimeAndVersion,
//             _ => MessageTypes::Unknown(cmd),
//         }
//     }
// }
//
// impl fmt::Display for MessageTypes {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         match *self {
//             MessageTypes::RegisterWorker => write!(f, "exMessageRegisterWorker"),
//             MessageTypes::SubmitShare => write!(f, "exMessageSubmitShare"),
//             MessageTypes::SubmitShareWithTime => write!(f, "exMessageSubmitShare"),
//             MessageTypes::UnregisterWorker => write!(f, "exMessageUnregisterWorker"),
//             MessageTypes::MiningSetDiff => write!(f, "exMessageMiningSetDiff"),
//             MessageTypes::SubmitShareWithVersion => write!(f, "exMessageSubmitShare"),
//             MessageTypes::SubmitShareWithTimeAndVersion => write!(f, "exMessageSubmitShare"),
//             _ => write!(f, ""),
//         }
//     }
// }
