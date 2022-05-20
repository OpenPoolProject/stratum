use crate::stratum_error::StratumError;
use std::net::IpAddr;
use uuid::Uuid;

//@todo review this. I'm not sure if this is the best strategy here, but I'd like for the ability
//to pass some information to the implementer of traits. I.E. For logging in, the implementer
//probably wants the IP of the connection that is attempting to log in, so that they can rate limit
//or ban. Further, for share submitting and other features, the implementer probably wants the same
//ability. Rather than pass the entire stream into the trait functions, I figured we should build a
//struct that holds some information about the miner that can be edited and passed into those
//functions.
#[derive(Clone, Debug)]
pub struct MinerInfo {
    pub ip: IpAddr,
    pub auth: Option<MinerAuth>,
    pub id: Option<Uuid>,
    pub sid: Option<String>,
    pub job_stats: Option<MinerJobStats>,
    pub name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct MinerAuth {
    pub id: String,
    pub username: String,
    pub client: String,
}

#[derive(Clone, Debug)]
pub struct MinerJobStats {
    pub expected_difficulty: f64,
}

impl MinerInfo {
    pub fn get_auth(&self) -> std::result::Result<MinerAuth, StratumError> {
        match &self.auth {
            Some(auth) => Ok(auth.clone()),
            None => Err(StratumError::Unauthorized),
        }
    }

    pub fn get_id(&self) -> std::result::Result<Uuid, StratumError> {
        match &self.id {
            Some(id) => Ok(*id),
            None => Err(StratumError::NotSubscribed),
        }
    }

    pub fn get_sid(&self) -> std::result::Result<String, StratumError> {
        match &self.sid {
            Some(sid) => Ok(sid.clone()),
            None => Err(StratumError::NotSubscribed),
        }
    }

    pub fn get_job_stats(&self) -> std::result::Result<MinerJobStats, StratumError> {
        match &self.job_stats {
            Some(job_stats) => Ok(job_stats.clone()),
            None => Err(StratumError::Internal),
        }
    }

    pub fn get_worker_name(&self) -> Option<String> {
        self.name.clone()
    }
}
