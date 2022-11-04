#![cfg_attr(coverage_nightly, feature(no_coverage))]
#![warn(clippy::pedantic)]

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

#[cfg(feature = "upstream")]
mod upstream;

#[cfg(feature = "upstream")]
use crate::config::UpstreamConfig;

#[cfg(feature = "api")]
mod api;

pub(crate) use crate::{ban_manager::BanManager, connection::Connection, frame::Frame};

pub use crate::{
    builder::StratumServerBuilder,
    config::{Config, ConfigManager, ConnectionConfig, DifficultyConfig},
    error::Error,
    global::Global,
    miner::Miner,
    miner_list::MinerList,
    request::StratumRequest,
    server::StratumServer,
    session::Session,
    session_list::SessionList,
    types::{ReadyIndicator, EX_MAGIC_NUMBER, ID},
    utils::format_difficulty,
};

pub type Result<T> = std::result::Result<T, Error>;
