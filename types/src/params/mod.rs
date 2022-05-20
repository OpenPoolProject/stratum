use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_tuple::{Deserialize_tuple, Serialize_tuple};

//@todo need some renaming on this bad boy.
use crate::traits::{PoolParams, StratumParams};

// #[derive(Debug, Clone, Deserialize, Serialize)]
// #[serde(untagged)]
// pub enum PoolParam<PP, SP>
// where
//     PP: PoolParams,
//     SP: StratumParams,
// {
//     Notify(SP::Notify),
//     SetDifficulty(SetDiff),

//     // ===== Responding Values ===== //
//     AuthorizeResult(PP::AuthorizeResult),
//     SubscribeResult(PP::SubscribeResult),
//     SubmitResult(bool),

//     Unknown(Value),
// }

// #[derive(Debug, Clone, Deserialize, Serialize)]
// #[serde(untagged)]
// pub enum ClientParam<PP, SP>
// where
//     PP: PoolParams,
//     SP: StratumParams,
// {
//     Authorize(PP::Authorize),
//     Submit(SP::Submit),
//     Subscribe(PP::Subscribe),
//     Unknown(Value),
// }

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Params<PP, SP>
where
    PP: PoolParams,
    SP: StratumParams,
{
    Authorize(PP::Authorize),
    Submit(SP::Submit),
    Subscribe(PP::Subscribe),
    Notify(SP::Notify),
    SetDifficulty(SetDiff),
    Unknown(Value),
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(untagged)]
pub enum Results<PP>
where
    PP: PoolParams,
{
    AuthorizeResult(PP::AuthorizeResult),
    SubscribeResult(PP::SubscribeResult),
    SubmitResult(bool),

    Unknown(Value),
}

//@todo make this a trait so we can use it in the above lib..
#[derive(Serialize_tuple, Deserialize_tuple, Clone, Debug)]
pub struct SetDiff {
    pub difficulty: f64,
}

// pub struct ClassicSetDiff {
//     difficulty: u64,
// }
