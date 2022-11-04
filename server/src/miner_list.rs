use crate::Miner;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

//Use dashmap
//@todo investigate better types at this -> I belive there is an async replacement for
//RwLockHashmap.
#[derive(Debug, Clone)]
pub struct MinerList {
    pub miners: Arc<RwLock<HashMap<u32, Miner>>>,
}

impl MinerList {
    #[must_use]
    pub fn new() -> Self {
        MinerList {
            miners: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn add_miner(&self, session_id: u32, miner: Miner) {
        self.miners.write().await.insert(session_id, miner);
    }

    //Returns the just removed miner (Mainly for Event reporting purposes)
    //If the miner exists otherwise returns None.
    pub async fn remove_miner(&self, session_id: u32) -> Option<Miner> {
        self.miners.write().await.remove(&(session_id))
    }

    pub async fn get_miner_by_id(&self, session_id: u32) -> Option<Miner> {
        self.miners.read().await.get(&(session_id)).cloned()
    }

    pub async fn update_miner_by_session_id(&self, session_id: u32, miner: Miner) {
        self.miners.write().await.insert(session_id, miner);
    }
}

impl Default for MinerList {
    fn default() -> Self {
        Self::new()
    }
}
