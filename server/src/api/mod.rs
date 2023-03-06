mod error;
mod routes;
mod server;
mod state;

pub use error::Error;
pub use server::Api;
pub use state::Context;

pub type Result<T> = std::result::Result<T, Error>;
