use crate::{types::SessionID, Miner};
use dashmap::DashMap;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct MinerList {
    pub miners: Arc<DashMap<SessionID, Miner>>,
}

impl MinerList {
    #[must_use]
    pub(crate) fn new() -> Self {
        MinerList {
            miners: Arc::new(DashMap::new()),
        }
    }

    pub(crate) fn add_miner(&self, session_id: SessionID, miner: Miner) {
        self.miners.insert(session_id, miner);
    }

    //Returns the just removed miner (Mainly for Event reporting purposes)
    //If the miner exists otherwise returns None.
    #[must_use]
    pub(crate) fn remove_miner(&self, session_id: SessionID) -> Option<(SessionID, Miner)> {
        self.miners.remove(&(session_id))
    }

    #[must_use]
    pub(crate) fn get_miner_by_id(&self, session_id: SessionID) -> Option<Miner> {
        self.miners.view(&session_id, |_k, v| v.clone())
    }

    pub(crate) fn update_miner_by_session_id(&self, session_id: SessionID, miner: Miner) {
        self.miners.insert(session_id, miner);
    }
}

impl Default for MinerList {
    fn default() -> Self {
        Self::new()
    }
}
