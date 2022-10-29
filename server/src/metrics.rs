use crate::MinerList;
use prometheus::{register_gauge, Gauge};
use std::sync::Arc;

#[derive(Clone)]
pub struct Metrics<CState: Clone + Sync + Send + 'static> {
    miner_list: Arc<MinerList>,
    pub(crate) total_connections: Gauge,
    pub(crate) time_since_last_job_broadcast: Gauge,
    pub(crate) last_job_broadcast_height: Gauge,
    pub(crate) shares_per_second: Gauge,
}

impl<CState: Clone + Sync + Send + 'static> Metrics<CState> {
    pub fn new(miner_list: Arc<MinerList<CState>>) -> Self {
        Metrics {
            miner_list,
            total_connections: register_gauge!("stratum_server_connections", "Server Connections")
                .unwrap(),
            time_since_last_job_broadcast: register_gauge!(
                "time_since_last_job_broadcast",
                "Job Idle Time"
            )
            .unwrap(),
            last_job_broadcast_height: register_gauge!(
                "last_job_broadcast_height",
                "Last Job Height"
            )
            .unwrap(),
            shares_per_second: register_gauge!("shares_per_second", "Shares Per Second").unwrap(),
        }
    }

    //@todo might also make more sense titlted "collect"
    pub async fn track(&self) {
        self.total_connections
            .set(self.miner_list.len().await as f64);

        //Iterate through every miner in the list and check
    }
}
