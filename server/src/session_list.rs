pub use crate::session::Session;
use crate::{ConfigManager, Result};
use extended_primitives::Buffer;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tokio::sync::RwLock;
use tracing::{info, warn};

//@todo performance test using a Sephamore for this similar to how Tokio does it in mini-redis

//@todo would love to get a data structure maybe stratumstats that is just recording all of the
//data and giving us some fucking baller output. Like shares/sec unit.
//
//Maybe total hashrate that we are processing at the moment.

//@todo would be nice to track the number of agent connections on here.
//@todo use improved RwLock<HashMap> libraries.
#[derive(Default, Clone)]
pub struct SessionList<CState: Clone + Sync + Send + 'static> {
    inner: Arc<Inner<CState>>,
    //@todo there are faster data structures than hashmap. Investigate using some of those.
    pub(crate) config_manager: ConfigManager,
}

#[derive(Default)]
struct Inner<CState> {
    state: RwLock<HashMap<SocketAddr, Arc<Session<CState>>>>,
}

impl<CState: Clone + Sync + Send + 'static> SessionList<CState> {
    #[must_use]
    pub fn new(config_manager: ConfigManager) -> Self {
        SessionList {
            inner: Arc::new(Inner {
                state: RwLock::new(HashMap::new()),
            }),
            config_manager,
        }
    }

    pub async fn add_miner(&self, addr: SocketAddr, miner: Arc<Session<CState>>) -> Result<()> {
        self.inner.state.write().await.insert(addr, miner);
        // gauge!(
        //     "stratum.num_connections",
        //     self.miners.read().await.len() as f64
        // );
        Ok(())
    }

    pub async fn remove_miner(&self, addr: SocketAddr) {
        self.inner.state.write().await.remove(&addr);
        // gauge!(
        //     "stratum.num_connections",
        //     self.miners.read().await.len() as f64
        // );
    }

    pub async fn get_all_miners(&self) -> Vec<Arc<Session<CState>>> {
        self.inner.state.read().await.values().cloned().collect()
    }

    pub async fn len(&self) -> usize {
        self.inner.state.read().await.len()
    }

    pub async fn is_empty(&self) -> bool {
        self.inner.state.read().await.is_empty()
    }

    pub async fn is_full(&self) -> bool {
        if let Some(max) = self
            .config_manager
            .current_config()
            .connection
            .max_connections
        {
            self.len().await >= max
        } else {
            false
        }
    }

    pub async fn shutdown_msg(&self, msg: Option<Buffer>) -> Result<()> {
        // @todo use this for deluge
        if let Some(msg) = msg {
            info!(
                "Session List sending {} miners reconnect message.",
                self.inner.state.read().await.len()
            );
            for miner in self.inner.state.read().await.values() {
                if let Err(e) = miner.send_raw(msg.clone()).await {
                    warn!(connection_id = %miner.id, cause = %e, "Failed to send shutdown message");
                }
            }
        }

        Ok(())
    }

    pub async fn shutdown(&self) {
        info!(
            "Session List shutting down {} miners",
            self.inner.state.read().await.len()
        );

        //@todo we need to parallize this async.
        for miner in self.inner.state.read().await.values() {
            miner.shutdown().await;
        }
    }
}
