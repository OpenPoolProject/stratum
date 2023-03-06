use crate::{ban_manager, session::SendInformation};
use futures::channel::mpsc::SendError;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("Banned connection attempted to connect: {0}")]
    ConnectionBanned(ban_manager::Key),
    #[error("Session IDs Exhausted")]
    SessionIDsExhausted,
    //This is the result of a non-graceful shutdown from someone connecting.
    #[error("Peer reset connection")]
    PeerResetConnection,
    #[error(transparent)]
    Sender(#[from] tokio::sync::mpsc::error::SendError<SendInformation>),
    #[error(transparent)]
    Json(#[from] serde_json::error::Error),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    MesssageSend(#[from] SendError),
    #[error(transparent)]
    AddrParseError(#[from] std::net::AddrParseError),
    #[cfg(feature = "api")]
    #[error(transparent)]
    API(#[from] crate::api::Error),

    //Non-updated Errors
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
    //@todo double cehck this covers it, and doesn't just feature gate the tranpsarent part.
    //@todo shutdown error.
    // #[error("Timeout Error: {0}")]
    // TimedOut(#[from] CancellationToken),
}
