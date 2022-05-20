pub use crate::connection::Connection;
use crate::Result;
use async_std::sync::{Arc, RwLock};
use std::{collections::HashMap, net::SocketAddr};

//@todo would love to get a data structure maybe stratumstats that is just recording all of the
//data and giving us some fucking baller output. Like shares/sec unit.
//
//Maybe total hashrate that we are processing at the moment.

//@todo would be nice to track the number of agent connections on here.
#[derive(Default)]
pub struct ConnectionList<CState: Clone + Sync + Send + 'static> {
    //@todo there are faster data structures than hashmap. Investigate using some of those.
    pub miners: RwLock<HashMap<SocketAddr, Arc<Connection<CState>>>>,
    pub max_connections: Option<usize>,
}

impl<CState: Clone + Sync + Send + 'static> ConnectionList<CState> {
    pub fn new(max_connections: Option<usize>) -> Self {
        ConnectionList {
            miners: RwLock::new(HashMap::new()),
            max_connections,
        }
    }

    pub async fn add_miner(&self, addr: SocketAddr, miner: Arc<Connection<CState>>) -> Result<()> {
        self.miners.write().await.insert(addr, miner);
        // gauge!(
        //     "stratum.num_connections",
        //     self.miners.read().await.len() as f64
        // );
        Ok(())
    }

    pub async fn remove_miner(&self, addr: SocketAddr) {
        self.miners.write().await.remove(&addr);
        // gauge!(
        //     "stratum.num_connections",
        //     self.miners.read().await.len() as f64
        // );
    }

    pub async fn get_all_miners(&self) -> Vec<Arc<Connection<CState>>> {
        self.miners.read().await.values().cloned().collect()
    }

    pub async fn len(&self) -> usize {
        self.miners.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.miners.read().await.is_empty()
    }

    pub async fn is_full(&self) -> bool {
        if let Some(max) = self.max_connections {
            self.len().await >= max
        } else {
            false
        }
    }

    pub async fn shutdown(&self) -> Result<()> {
        //@todo we need to parallize this async.
        for miner in self.miners.read().await.values() {
            miner.shutdown().await;
        }

        Ok(())
    }
}
