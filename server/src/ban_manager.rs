use dashmap::DashMap;
use futures::StreamExt;
use serde::Serialize;
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tokio::sync::{Mutex, Notify};
use tokio_util::{
    sync::CancellationToken,
    time::delay_queue::{DelayQueue, Key},
};
use tracing::info;

/// A wrapping around entries that adds the link to the entry's expiration, via a `delay_queue` key.
#[derive(Debug)]
struct MapEntry<V> {
    /// The expiration key for the entry.
    key: Key,
    /// The actual entry.
    value: V,
}

#[derive(Serialize, Clone, Debug)]
pub struct BanInfo {
    pub address: SocketAddr,
    pub score: u64,
}

//@todo perma bans
#[derive(Debug, Default)]
pub struct BanManager {
    pub(crate) shared: Arc<Shared>,
    default_ban_length: Duration,
}

#[derive(Default, Debug)]
pub(crate) struct Shared {
    pub(crate) state: Mutex<State>,
    pub(crate) cancel_token: CancellationToken,
    background_task: Notify,
    temp_bans: Arc<DashMap<SocketAddr, MapEntry<BanInfo>>>,
}

#[derive(Default, Debug)]
pub(crate) struct State {
    pub(crate) expirations: DelayQueue<SocketAddr>,
}

impl BanManager {
    pub fn new(cancel_token: CancellationToken, default_ban_length: Duration) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                expirations: DelayQueue::new(),
            }),
            temp_bans: Arc::new(DashMap::new()),
            background_task: Notify::new(),
            cancel_token,
        });

        tokio::spawn(purge_ban_manager(shared.clone()));
        // 1 hour
        BanManager {
            shared,
            default_ban_length,
        }
    }

    pub fn check_banned(&self, addr: &SocketAddr) -> bool {
        self.shared.temp_bans.contains_key(addr)
    }

    //@todo figure out what score means
    pub async fn add_ban(&self, addr: &SocketAddr) {
        self.add_ban_score(addr, 10).await;
    }

    pub async fn add_ban_score(&self, addr: &SocketAddr, score: u64) {
        //@todo there may be other's we want to check for here.
        //@todo we probably want to have an IP whitelist on here.
        if addr.ip().is_loopback() || addr.ip().is_unspecified() {
            return;
        }

        let mut state = self.shared.state.lock().await;

        if let Some(entry) = self.shared.temp_bans.get(addr) {
            state.expirations.reset(&entry.key, self.default_ban_length);
        } else {
            let key = state.expirations.insert(*addr, self.default_ban_length);

            let value = BanInfo {
                address: *addr,
                score,
            };

            let entry = MapEntry { key, value };

            self.shared.temp_bans.insert(*addr, entry);
            drop(state);

            self.shared.background_task.notify_one();
        }
    }

    // #[cfg(feature = "api")]
    /// Returns a vector of referencing all values in the map.
    pub fn temp_bans(&self) -> Vec<BanInfo> {
        self.shared
            .temp_bans
            .iter()
            .map(|ref_multi| ref_multi.value().value.clone())
            .collect()
    }
}

impl Drop for BanManager {
    fn drop(&mut self) {
        self.shared.cancel_token.cancel();
        self.shared.background_task.notify_one();
    }
}

async fn purge_ban_manager(shared: Arc<Shared>) {
    let shared = shared.clone();

    while !shared.cancel_token.is_cancelled() {
        if !shared.state.lock().await.expirations.is_empty() {
            let mut state = shared.state.lock().await;

            tokio::select! {
                res = state.expirations.next() => {
                    if let Some(expired) = res {
                            let addr = expired.into_inner();
                            shared.temp_bans.remove(&addr);
                    }
                }
                _ = shared.background_task.notified() => {}
            }
        } else {
            shared.background_task.notified().await;
        }
    }

    info!("Ban Manager shutting down.")
}

pub type BanManagerHandle = Arc<BanManager>;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use tokio_test::assert_ok;

    #[tokio::test]
    async fn single_ban_expires() {
        let cancel_token = CancellationToken::new();
        let ban_manager = BanManager::new(cancel_token, ms(1));

        ban_manager
            .add_ban(&assert_ok!("163.244.101.203:3821".parse()))
            .await;

        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 1);

        tokio::time::sleep(ms(10)).await;

        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 0);
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[tokio::test]
    async fn graceful_shutdown() {
        let cancel_token = CancellationToken::new();
        let ban_manager = BanManager::new(cancel_token.child_token(), ms(100));

        let addr = assert_ok!(SocketAddr::from_str("163.244.101.203:3821"));

        ban_manager.add_ban(&addr).await;

        assert!(ban_manager.check_banned(&addr));

        cancel_token.cancel();

        tokio::time::sleep(ms(200)).await;

        assert!(ban_manager.check_banned(&addr));
    }
}
