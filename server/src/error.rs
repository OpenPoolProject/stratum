use futures::channel::mpsc::SendError;

//@todo transparent all of these
#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Stratum User not authorized")]
    NotAuthorized,
    #[error("Stratum Stream Closed. Reasion: {0}")]
    StreamClosed(String),
    #[error("Connection used wrong port in proxy protoocl")]
    StreamWrongPort,
    #[error("Method does not exist")]
    MethodDoesntExist,
    #[error("Can't break ExMessage header - Not complete")]
    BrokenExHeader,
    #[error("Json Error: {0}")]
    Json(#[from] serde_json::error::Error),
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Channel Send Error: {0}")]
    MesssageSend(#[from] SendError),
    #[error("Address Parse Error: {0}")]
    AddrParseError(#[from] std::net::AddrParseError),
    //@todo double cehck this covers it, and doesn't just feature gate the tranpsarent part.
    #[cfg(feature = "api")]
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    //@todo shutdown error.
    // #[error("Timeout Error: {0}")]
    // TimedOut(#[from] CancellationToken),
}
