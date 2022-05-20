use crate::stratum_error::StratumError;
use futures::channel::mpsc::SendError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotAuthorized,
    StreamClosed,
    MethodDoesntExist,
    Json(serde_json::error::Error),
    Io(std::io::Error),
    MesssageSend(SendError),
    Stratum(StratumError),
}

impl From<StratumError> for Error {
    fn from(e: StratumError) -> Error {
        Error::Stratum(e)
    }
}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<serde_json::error::Error> for Error {
    fn from(e: serde_json::error::Error) -> Error {
        Error::Json(e)
    }
}

impl From<SendError> for Error {
    fn from(e: SendError) -> Error {
        Error::MesssageSend(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NotAuthorized => write!(f, "Stratum User not authorized"),
            Error::StreamClosed => write!(f, "Stratum Stream closed"),
            Error::MethodDoesntExist => write!(f, "Stratum Method Doesn't Exist"),
            Error::Json(ref e) => write!(f, "JSON error: {}", e),
            Error::Io(ref e) => write!(f, "IO Error: {}", e),
            Error::MesssageSend(ref e) => write!(f, "Channel Send Error: {}", e),
            Error::Stratum(ref e) => write!(f, "Stratum Error: {}", e),
        }
    }
}
