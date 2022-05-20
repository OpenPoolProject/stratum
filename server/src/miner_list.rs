use crate::Miner;
use async_std::sync::{Arc, RwLock};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MinerList {
    pub miners: Arc<RwLock<HashMap<u32, Miner>>>,
}

impl MinerList {
    pub fn new() -> Self {
        MinerList {
            miners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_miner(&self, session_id: u32, miner: Miner) {
        self.miners.write().await.insert(session_id as u32, miner);
    }

    //Returns the just removed miner (Mainly for Event reporting purposes)
    //If the miner exists otherwise returns None.
    pub async fn remove_miner(&self, session_id: u32) -> Option<Miner> {
        self.miners.write().await.remove(&(session_id as u32))
    }

    pub async fn get_miner_by_id(&self, session_id: u32) -> Option<Miner> {
        self.miners.read().await.get(&(session_id as u32)).cloned()
    }

    pub async fn update_miner_by_session_id(&self, session_id: u32, miner: Miner) {
        self.miners.write().await.insert(session_id as u32, miner);
    }
}

impl Default for MinerList {
    fn default() -> Self {
        Self::new()
    }
}
