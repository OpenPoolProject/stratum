//@todo let's check to see if we can remove this.
#![recursion_limit = "512"]

#[warn(clippy::pedantic)]
mod tcp;

#[cfg(feature = "upstream")]
mod upstream;

#[cfg(feature = "upstream")]
use crate::config::UpstreamConfig;

// mod api;
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
mod types;
mod utils;

pub use crate::{
    ban_manager::BanManager,
    builder::StratumServerBuilder,
    config::VarDiffConfig,
    connection::Connection,
    connection_list::ConnectionList,
    error::Error,
    global::Global,
    miner::Miner,
    miner_list::MinerList,
    parsing::next_message,
    request::StratumRequest,
    server::StratumServer,
    types::{ExMessageGeneric, MessageTypes, MessageValue, ReadyIndicator, EX_MAGIC_NUMBER, ID},
    utils::format_difficulty,
};

pub type Result<T> = std::result::Result<T, Error>;
