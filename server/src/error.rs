use futures::channel::mpsc::SendError;
use std::fmt;

#[derive(Debug)]
pub enum Error {
    NotAuthorized,
    StreamClosed(String),
    StreamWrongPort,
    MethodDoesntExist,
    BrokenExHeader,
    #[cfg(feature = "websockets")]
    Websocket(async_tungstenite::tungstenite::Error),
    Json(serde_json::error::Error),
    Io(std::io::Error),
    MesssageSend(SendError),
    AddrParseError(std::net::AddrParseError),
    TimedOut(stop_token::TimedOutError),
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

impl From<std::net::AddrParseError> for Error {
    fn from(e: std::net::AddrParseError) -> Error {
        Error::AddrParseError(e)
    }
}

impl From<stop_token::TimedOutError> for Error {
    fn from(e: stop_token::TimedOutError) -> Error {
        Error::TimedOut(e)
    }
}

#[cfg(feature = "websockets")]
impl From<async_tungstenite::tungstenite::Error> for Error {
    fn from(e: async_tungstenite::tungstenite::Error) -> Error {
        Error::Websocket(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::NotAuthorized => write!(f, "Stratum User not authorized"),
            Error::StreamClosed(ref e) => write!(f, "Stratum Stream closed. Reason: {}", e),
            Error::StreamWrongPort => write!(f, "Stratum Stream used wrong port in proxy protocol"),
            Error::MethodDoesntExist => write!(f, "Stratum Method Doesn't Exist"),
            Error::BrokenExHeader => write!(f, "Can't break Ex Message header not complete."),
            Error::Json(ref e) => write!(f, "JSON error: {}", e),
            Error::Io(ref e) => write!(f, "IO Error: {}", e),
            Error::MesssageSend(ref e) => write!(f, "Channel Send Error: {}", e),
            Error::AddrParseError(ref e) => write!(f, "Address Parsing Error: {}", e),
            Error::TimedOut(ref e) => write!(f, "Timed Out Error: {}", e),
            #[cfg(feature = "websockets")]
            Error::Websocket(ref e) => write!(f, "Websocket Error: {}", e),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            Error::NotAuthorized => None,
            Error::StreamClosed(ref _e) => None,
            Error::StreamWrongPort => None,
            Error::MethodDoesntExist => None,
            Error::BrokenExHeader => None,
            Error::Json(ref e) => Some(e),
            Error::Io(ref e) => Some(e),
            Error::MesssageSend(ref e) => Some(e),
            Error::AddrParseError(ref e) => Some(e),
            Error::TimedOut(ref e) => Some(e),
            #[cfg(feature = "websockets")]
            Error::Websocket(ref e) => Some(e),
        }
    }
}
