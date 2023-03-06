use crate::{session::Session, ConfigManager, Result};
use dashmap::DashMap;
use extended_primitives::Buffer;
use std::{net::SocketAddr, sync::Arc};
use tracing::{info, warn};

//@todo performance test using a Sephamore for this similar to how Tokio does it in mini-redis

//@todo would love to get a data structure maybe stratumstats that is just recording all of the
//data and giving us some fucking baller output. Like shares/sec unit.
//
//Maybe total hashrate that we are processing at the moment.

//@todo would be nice to track the number of agent connections on here.
//@todo use improved RwLock<HashMap> libraries.
#[derive(Default, Clone)]
pub struct SessionList<CState: Clone> {
    inner: Arc<Inner<CState>>,
    pub(crate) config_manager: ConfigManager,
}

#[derive(Default)]
struct Inner<CState> {
    state: DashMap<SocketAddr, Session<CState>>,
}

impl<CState: Clone> SessionList<CState> {
    #[must_use]
    pub fn new(config_manager: ConfigManager) -> Self {
        SessionList {
            inner: Arc::new(Inner {
                state: DashMap::new(),
            }),
            config_manager,
        }
    }

    pub fn add_miner(&self, addr: SocketAddr, miner: Session<CState>) -> Result<()> {
        self.inner.state.insert(addr, miner);
        // gauge!(
        //     "stratum.num_connections",
        //     self.miners.read().await.len() as f64
        // );
        Ok(())
    }

    pub fn remove_miner(&self, addr: SocketAddr) {
        self.inner.state.remove(&addr);
        // gauge!(
        //     "stratum.num_connections",
        //     self.miners.read().await.len() as f64
        // );
    }

    #[must_use]
    pub fn get_all_miners(&self) -> Vec<Session<CState>> {
        self.inner.state.iter().map(|x| x.value().clone()).collect()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.inner.state.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.inner.state.is_empty()
    }

    #[must_use]
    pub fn is_full(&self) -> bool {
        if let Some(max) = self
            .config_manager
            .current_config()
            .connection
            .max_connections
        {
            self.len() >= max
        } else {
            false
        }
    }

    //@todo we need to revamp this as it needs to be variable.
    pub fn shutdown_msg(&self, msg: Option<Buffer>) -> Result<()> {
        // @todo use this for deluge
        if let Some(msg) = msg {
            info!(
                "Session List sending {} miners reconnect message.",
                self.inner.state.len()
            );
            for entry in self.inner.state.iter() {
                let miner = entry.value();
                if let Err(e) = miner.send_raw(msg.clone()) {
                    warn!(connection_id = %miner.id(), cause = %e, "Failed to send shutdown message");
                }
            }
        }

        Ok(())
    }

    pub fn shutdown(&self) {
        info!(
            "Session List shutting down {} miners",
            self.inner.state.len()
        );

        //@todo we need to parallize this async - now we can do it without async though.
        for entry in self.inner.state.iter() {
            entry.value().shutdown();
        }
    }
}
