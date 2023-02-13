use crate::Miner;
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MinerList {
    pub miners: Arc<DashMap<u32, Miner>>,
}

impl MinerList {
    #[must_use]
    pub fn new() -> Self {
        MinerList {
            miners: Arc::new(DashMap::new()),
        }
    }

    pub async fn add_miner(&self, session_id: u32, miner: Miner) {
        self.miners.insert(session_id, miner);
    }

    //Returns the just removed miner (Mainly for Event reporting purposes)
    //If the miner exists otherwise returns None.
    pub async fn remove_miner(&self, session_id: u32) -> Option<(u32, Miner)> {
        self.miners.remove(&(session_id))
    }

    pub async fn get_miner_by_id(&self, session_id: u32) -> Option<Miner> {
        self.miners.view(&session_id, |_k, v| v.clone())
    }

    pub async fn update_miner_by_session_id(&self, session_id: u32, miner: Miner) {
        self.miners.insert(session_id, miner);
    }
}

impl Default for MinerList {
    fn default() -> Self {
        Self::new()
    }
}
