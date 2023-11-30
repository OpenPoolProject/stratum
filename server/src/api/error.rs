#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error(transparent)]
    Hyper(#[from] hyper::Error),
    #[error(transparent)]
    AddressParseError(#[from] std::net::AddrParseError),
    #[error(transparent)]
    Io(#[from] std::io::Error),
}
