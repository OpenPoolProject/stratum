use dashmap::{mapref::one::RefMut, DashMap};
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::Display,
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{
    sync::Notify,
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, warn};

use crate::{Error, Result};

#[derive(Hash, PartialEq, Eq, Debug, Clone, Serialize, Deserialize)]
pub enum Key {
    IP(IpAddr),
    Socket(SocketAddr),
    // Account(Username)
    Account(String),
    // Account(Username)
    Worker(String),
}

impl Display for Key {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Key::IP(ip) => write!(f, "IP: {ip}"),
            Key::Socket(socket) => write!(f, "Socket: {socket}"),
            Key::Account(account) => write!(f, "Account: {account}"),
            //@todo do this better when its account, workername
            Key::Worker(worker) => write!(f, "Worker: {worker}"),
        }
    }
}

impl From<SocketAddr> for Key {
    fn from(value: SocketAddr) -> Self {
        Key::Socket(value)
    }
}

impl From<IpAddr> for Key {
    fn from(value: IpAddr) -> Self {
        Key::IP(value)
    }
}

//@todo implement froms here.

/// A wrapping around entries that adds the link to the entry's expiration, via a `delay_queue` key.
#[derive(Debug)]
struct Entry {
    /// Uniquely identifies this entry.
    id: u64,

    /// Stored data
    data: BanInfo,

    /// Instant at which the entry expires and should be removed from the
    /// database.
    expires_at: Instant,
}

#[derive(Serialize, Clone, Debug)]
pub struct BanInfo {
    pub address: Key,
    pub score: u64,
}

//@todo perma bans
pub struct BanManager {
    pub(crate) shared: Arc<Shared>,
    default_ban_length: Duration,
}

pub(crate) struct Shared {
    pub(crate) state: Mutex<State>,
    pub(crate) cancel_token: CancellationToken,
    background_task: Notify,
    temp_bans: Arc<DashMap<Key, Entry>>,
}

impl Shared {
    /// Purge all expired keys and return the `Instant` at which the **next**
    /// key will expire. The background task will sleep until this instant.
    fn purge_expired_keys(&self) -> Option<Instant> {
        if self.cancel_token.is_cancelled() {
            // The database is shutting down. All handles to the shared state
            // have dropped. The background task should exit.
            return None;
        }

        let mut state = self.state.lock();

        // This is needed to make the borrow checker happy. In short, `lock()`
        // returns a `MutexGuard` and not a `&mut State`. The borrow checker is
        // not able to see "through" the mutex guard and determine that it is
        // safe to access both `state.expirations` and `state.entries` mutably,
        // so we get a "real" mutable reference to `State` outside of the loop.
        // let state = &mut *state;

        // Find all keys scheduled to expire **before** now.
        let now = Instant::now();

        while let Some((&(when, id), key)) = state.expirations.iter().next() {
            if when > now {
                // Done purging, `when` is the instant at which the next key
                // expires. The worker task will wait until this instant.
                return Some(when);
            }

            // The key expired, remove it
            self.temp_bans.remove(key);
            state.expirations.remove(&(when, id));
        }

        None
    }
}

/// Routine executed by the background task.
///
/// Wait to be notified. On notification, purge any expired keys from the shared
/// state handle. If `shutdown` is set, terminate the task.
async fn purge_expired_tasks(shared: Arc<Shared>) {
    // If the shutdown flag is set, then the task should exit.
    while !shared.cancel_token.is_cancelled() {
        // Purge all keys that are expired. The function returns the instant at
        // which the **next** key will expire. The worker should wait until the
        // instant has passed then purge again.
        if let Some(when) = shared.purge_expired_keys() {
            // Wait until the next key expires **or** until the background task
            // is notified. If the task is notified, then it must reload its
            // state as new keys have been set to expire early. This is done by
            // looping.
            tokio::select! {
                _ = tokio::time::sleep_until(when) => {}
                _ = shared.background_task.notified() => {}
            }
        } else {
            // There are no keys expiring in the future. Wait until the task is
            // notified.
            shared.background_task.notified().await;
        }
    }

    debug!("Purge background task shut down");
}

#[derive(Default)]
pub(crate) struct State {
    // pub(crate) expirations: DelayQueue<SocketAddr>,
    /// Tracks key TTLs.
    ///
    /// A `BTreeMap` is used to maintain expirations sorted by when they expire.
    /// This allows the background task to iterate this map to find the value
    /// expiring next.
    ///
    /// While highly unlikely, it is possible for more than one expiration to be
    /// created for the same instant. Because of this, the `Instant` is
    /// insufficient for the key. A unique expiration identifier (`u64`) is used
    /// to break these ties.
    expirations: BTreeMap<(Instant, u64), Key>,

    /// Identifier to use for the next expiration. Each expiration is associated
    /// with a unique identifier. See above for why.
    next_id: u64,
}

impl State {
    fn next_expiration(&self) -> Option<Instant> {
        self.expirations
            .keys()
            .next()
            .map(|expiration| expiration.0)
    }
}

//@todo 1. Add remove_ban, and feature gate it for the API -> That way we can unban miners manually
//if we need to (or IP addresses).
//2. Feature gate all of Ban Manager,
//3. Allow for white_list or non-bannables
//4. Allow for not just IPs to be banned, but usernames, and usnermae/workernames combinations.
//- For the above can we switch to using an Enum as the hashmpa key
impl BanManager {
    pub fn new(cancel_token: CancellationToken, default_ban_length: Duration) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            temp_bans: Arc::new(DashMap::new()),
            background_task: Notify::new(),
            cancel_token,
        });

        tokio::spawn(purge_expired_tasks(shared.clone()));
        // 1 hour
        BanManager {
            shared,
            default_ban_length,
        }
    }

    pub fn check_banned<T: Into<Key>>(&self, key: T) -> Result<()> {
        let key = key.into();
        if self.shared.temp_bans.contains_key(&key) {
            Err(Error::ConnectionBanned(key))
        } else {
            Ok(())
        }
    }

    //@todo figure out what score means
    pub fn add_ban<T: Into<Key>>(&self, key: T) {
        self.add_ban_raw(&key.into(), 10, self.default_ban_length);
    }

    //@todo add_ban generic that impls Into<Key>

    fn add_ban_raw(&self, key: &Key, score: u64, dur: Duration) {
        //@todo there may be other's we want to check for here.
        //@todo we probably want to have an IP whitelist on here.
        //@todo move these to ban socket or ip.
        // if addr.ip().is_loopback() || addr.ip().is_unspecified() {
        //     return;
        // }

        let mut state = self.shared.state.lock();

        // Get and increment the next insertion ID. Guarded by the lock, this
        // ensures a unique identifier is associated with each `set` operation.
        let id = state.next_id;
        state.next_id += 1;

        let expires_at = Instant::now() + dur;

        // Only notify the worker task if the newly inserted expiration is the
        // **next** key to evict. In this case, the worker needs to be woken up
        // to update its state.
        let notify = state
            .next_expiration()
            .map_or(true, |expiration| expiration > expires_at);

        // Track the expiration.
        state.expirations.insert((expires_at, id), key.clone());

        //@todo might make sense to drop state here, before jumping into Dashmap potential locking
        //scenario.
        drop(state);

        if let Some(entry) = self.shared.temp_bans.get_mut(key) {
            // let old_entry = entry;

            let mut state = self.shared.state.lock();
            //Remove the old expiration as we've set a new one.
            state.expirations.remove(&(entry.expires_at, entry.id));

            let new_score = entry.data.score + score;

            let mut new_entry = RefMut::map(entry, |t| t);

            //@todo test if this works.
            new_entry.data.score = new_score;
            new_entry.id = id;

            drop(state);
        } else {
            let entry = Entry {
                id,
                data: BanInfo {
                    address: key.clone(),
                    score,
                },
                expires_at,
            };

            self.shared.temp_bans.insert(key.clone(), entry);
        }

        if notify {
            self.shared.background_task.notify_one();
        }
    }

    pub fn remove_ban<T: Into<Key>>(&self, key: T) -> Option<BanInfo> {
        let mut state = self.shared.state.lock();
        let key = key.into();

        if let Some((_, entry)) = self.shared.temp_bans.remove(&key) {
            warn!("Manually unbanning: {key}. Make sure you know what you are doing!");
            state.expirations.remove(&(entry.expires_at, entry.id));
            return Some(entry.data);
        }

        None
    }

    // #[cfg(feature = "api")]
    /// Returns a vector of referencing all values in the map.
    pub fn temp_bans(&self) -> Vec<BanInfo> {
        self.shared
            .temp_bans
            .iter()
            .map(|ref_multi| ref_multi.value().data.clone())
            .collect()
    }
}

impl Drop for BanManager {
    fn drop(&mut self) {
        self.shared.cancel_token.cancel();
        self.shared.background_task.notify_one();
    }
}

pub type Handle = Arc<BanManager>;

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use tokio_test::{assert_err, assert_ok};

    #[cfg_attr(coverage_nightly, no_coverage)]
    #[tokio::test]
    async fn single_ban_expires() {
        let cancel_token = CancellationToken::new();
        let ban_manager = BanManager::new(cancel_token, ms(1));

        let bad_miner: SocketAddr = assert_ok!("163.244.101.203:3841".parse());

        ban_manager.add_ban(bad_miner);

        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 1);

        tokio::time::sleep(ms(10)).await;

        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 0);
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    #[tokio::test]
    async fn ban_extended() {
        let cancel_token = CancellationToken::new();
        let ban_manager = BanManager::new(cancel_token, Duration::from_secs(100));

        // tokio::time::pause();

        let bad_miner: SocketAddr = assert_ok!("163.244.101.203:3841".parse());

        ban_manager.add_ban(bad_miner);

        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 1);
        // assert_eq!(temp_bans[0].address, bad_miner);
        assert_eq!(temp_bans[0].score, 10);

        // tokio::time::advance(ms(10)).await;
        // tokio::time::sleep(ms(10)).await;
        // tokio::time::pau

        ban_manager.add_ban(bad_miner);

        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 1);
        // assert_eq!(temp_bans[0].address, bad_miner);
        assert_eq!(temp_bans[0].score, 20);

        tokio::time::sleep(ms(40)).await;

        ban_manager.remove_ban(bad_miner);
        let temp_bans = ban_manager.temp_bans();

        assert_eq!(temp_bans.len(), 0);
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    #[tokio::test]
    async fn graceful_shutdown() {
        let cancel_token = CancellationToken::new();
        let ban_manager = BanManager::new(cancel_token.child_token(), ms(100));

        let addr = assert_ok!(SocketAddr::from_str("163.244.101.203:3821"));

        ban_manager.add_ban(addr);

        assert_err!(ban_manager.check_banned(addr));

        cancel_token.cancel();

        tokio::time::sleep(ms(200)).await;

        assert_err!(ban_manager.check_banned(addr));
    }
}
