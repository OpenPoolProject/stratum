pub use crate::connection::Connection;
use crate::Result;
use extended_primitives::Buffer;
use std::{collections::HashMap, net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::RwLock;
use tracing::{info, warn};

//@todo would love to get a data structure maybe stratumstats that is just recording all of the
//data and giving us some fucking baller output. Like shares/sec unit.
//
//Maybe total hashrate that we are processing at the moment.

//@todo would be nice to track the number of agent connections on here.
//@todo use improved RwLock<HashMap> libraries.
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

    pub async fn shutdown(&self, msg: Option<Buffer>, delay_seconds: u64) -> Result<()> {
        // First we send the miners a message (if provided), and give them a few seconds to
        // reconnect to the new proxy.
        // @todo move this into a separate function since we will want to call this without always
        // shutting down (via API)
        if let Some(msg) = msg {
            info!(
                "Connection List sending {} miners reconnect message.",
                self.miners.read().await.len()
            );
            for miner in self.miners.read().await.values() {
                //@todo we don't want to throw as it would prevent other miners from getting the
                //message.
                //@todo log error here btw.
                match miner.send_raw(msg.clone()).await {
                    Ok(_) => {}
                    //@todo fix this, causing recursion limits
                    // Err(_) => warn!(connection_id = %miner.id, "Failed to send shutdown message"),
                    Err(_) => warn!("Failed to send shutdown message"),
                }
            }
            //@todo log
            //All miners have received the shutdown message, now we wait and then we remove.
            tokio::time::sleep(Duration::from_secs(delay_seconds)).await;
        }

        info!(
            "Connection List shutting down {} miners",
            self.miners.read().await.len()
        );

        //@todo we need to parallize this async.
        for miner in self.miners.read().await.values() {
            miner.shutdown().await;
        }

        Ok(())
    }
}
