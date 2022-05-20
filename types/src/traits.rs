use crate::miner::MinerInfo;
use crate::stratum_error::StratumError;
use crate::Result;
use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

pub trait SubscribeResult: DeserializeOwned + Serialize + Sync + Send + Clone + Debug {
    fn id(&self) -> Option<Uuid>;
    fn sid(&self) -> String;
}

pub trait AuthorizeResult: DeserializeOwned + Serialize + Sync + Send + Clone + Debug {
    fn authorized(&self) -> bool;
    fn id(&self) -> String;
    fn worker_id(&self) -> Option<Uuid>;
    fn worker_name(&self) -> Option<String>;
}

pub trait Authorize: DeserializeOwned + Serialize + Sync + Send + Clone + Debug {
    fn username(&self) -> String;
    fn client(&self) -> String;
    fn worker_name(&self) -> Option<String>;
}

pub trait Notify: DeserializeOwned + Serialize + Sync + Send + Clone + Debug {
    fn get_difficulty(&self) -> f64;
}

pub trait Subscribe: DeserializeOwned + Serialize + Sync + Send + Clone + Debug {
    fn set_sid(&mut self, sid: &str);
}

pub trait Submit: DeserializeOwned + Serialize + Sync + Send + Clone + Debug {
    fn username(&self) -> String;
}

pub trait StratumParams {
    type Submit: Submit;
    type Notify: Notify;
}

pub trait PoolParams {
    type Authorize: Authorize;
    type AuthorizeResult: AuthorizeResult;
    type Subscribe: Subscribe;
    type SubscribeResult: SubscribeResult;
}

#[async_trait]
pub trait StratumManager: Sync + Send + Clone {
    type StratumParams: StratumParams + DeserializeOwned + Serialize + Clone;
    type PoolParams: PoolParams + DeserializeOwned + Serialize + Clone;

    type AuthManager: AuthManager<
        Authorize = <Self::PoolParams as PoolParams>::Authorize,
        AuthorizeResult = <Self::PoolParams as PoolParams>::AuthorizeResult,
        Subscribe = <Self::PoolParams as PoolParams>::Subscribe,
        SubscribeResult = <Self::PoolParams as PoolParams>::SubscribeResult,
    >;
    type DataProvider: DataProvider<Job = <Self::StratumParams as StratumParams>::Notify>;
    type BlockValidator: BlockValidator<
        Job = <Self::StratumParams as StratumParams>::Notify,
        Submit = <Self::StratumParams as StratumParams>::Submit,
    >;
}

//@todo note to self. Ideally, we actually switch this entire implementation to send a pointer of
//the actual miner to this trait. That way we can just pull stats from the miner itself, but this
//will be in a later minor/major version.
#[async_trait]
pub trait AuthManager: Sync + Send + Clone {
    type Authorize: Authorize;
    type AuthorizeResult: AuthorizeResult;
    type Subscribe: Subscribe;
    type SubscribeResult: SubscribeResult;

    async fn authorize(
        &self,
        info: MinerInfo,
        auth: &Self::Authorize,
        classic: bool,
    ) -> std::result::Result<Self::AuthorizeResult, StratumError>;

    async fn authorize_username(
        &self,
        info: MinerInfo,
        auth: &str,
    ) -> std::result::Result<Self::AuthorizeResult, StratumError>;

    async fn subscribe(
        &self,
        info: MinerInfo,
        subscribe_info: &Self::Subscribe,
        classic: bool,
    ) -> std::result::Result<Self::SubscribeResult, StratumError>;
}

#[async_trait]
pub trait DataProvider: Sync + Send + Clone {
    type Job: Notify;

    //Init allows the data provider to do whatever it needs before starting. In the case of an RPC
    //client, this would mean making the initial connection to the rpc server and getting the first
    //block template.
    async fn init(&self) -> Result<()>;

    async fn get_job(&self, classic: bool) -> Self::Job;

    //@todo polls for new templates. If it returns true, we then there is a new block template
    //available. Might want to optimize this slightly.
    async fn poll_template(&self) -> std::result::Result<bool, StratumError>;

    //@todo review this.
    // async fn get_tx_data(&self, timestamp: u64) -> Result<Vec<Self::TransactionData>>;
}

#[async_trait]
pub trait BlockValidator: Sync + Send + Clone {
    type Submit: Submit;
    type Job: DeserializeOwned + Serialize + Sync + Send + Clone + Debug;

    //Init allows the data provider to do whatever it needs before starting. In the case of an RPC
    //client, this would mean making the initial connection to the rpc server and getting the first
    //block template.
    async fn init(&self) -> Result<()>;

    //Handles validating share, and submitting
    async fn validate_share(
        &self,
        info: MinerInfo,
        share: Self::Submit,
        classic: bool,
    ) -> std::result::Result<bool, StratumError>;
}
