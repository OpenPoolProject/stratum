#![cfg_attr(coverage_nightly, feature(no_coverage))]
#[warn(clippy::pedantic)]
#[cfg(feature = "upstream")]
mod upstream;

#[cfg(feature = "upstream")]
use crate::config::UpstreamConfig;

#[cfg(feature = "api")]
mod api;

mod ban_manager;
mod builder;
mod config;
mod connection;
mod connection_list;
mod error;
mod global;
mod id_manager;
mod miner;
mod miner_list;
mod parsing;
mod request;
mod route;
mod router;
mod server;
mod tcp;
mod types;
mod utils;

pub(crate) use crate::ban_manager::BanManager;

pub use crate::{
    builder::StratumServerBuilder,
    config::VarDiffConfig,
    connection::Connection,
    connection_list::ConnectionList,
    error::Error,
    global::Global,
    miner::Miner,
    miner_list::MinerList,
    request::StratumRequest,
    server::StratumServer,
    types::{ExMessageGeneric, MessageTypes, MessageValue, ReadyIndicator, EX_MAGIC_NUMBER, ID},
    utils::format_difficulty,
};

pub type Result<T> = std::result::Result<T, Error>;
