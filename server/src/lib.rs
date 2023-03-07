#![cfg_attr(coverage_nightly, feature(no_coverage))]
#![warn(clippy::pedantic)]
#![allow(clippy::module_name_repetitions)]
//@todo fix this.
#![allow(clippy::missing_errors_doc)]
//@todo fix this.
#![allow(clippy::missing_panics_doc)]
//@todo remove eventually
#![allow(clippy::cast_lossless)]
//@todo remove eventually
#![allow(clippy::cast_precision_loss)]

mod ban_manager;
mod builder;
mod config;
mod connection;
mod error;
mod frame;
mod global;
mod id_manager;
mod miner;
mod miner_list;
mod request;
mod route;
mod router;
mod server;
mod session;
mod session_list;
mod tcp;
mod types;
mod utils;

// #[cfg(feature = "upstream")]
// mod upstream;

// #[cfg(feature = "upstream")]
// use crate::config::UpstreamConfig;

#[cfg(feature = "api")]
mod api;

pub(crate) use crate::{
    ban_manager::BanManager, connection::Connection, frame::Frame, miner_list::MinerList,
};

pub use crate::{
    builder::StratumServerBuilder,
    config::{Config, ConfigManager, ConnectionConfig, DifficultyConfig},
    error::Error,
    global::Global,
    miner::Miner,
    request::StratumRequest,
    server::StratumServer,
    session::Session,
    session_list::SessionList,
    types::{ReadyIndicator, SessionID, EX_MAGIC_NUMBER, ID},
};

pub type Result<T> = std::result::Result<T, Error>;
