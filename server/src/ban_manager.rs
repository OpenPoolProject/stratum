pub use crate::connection::Connection;
use async_std::sync::RwLock;
use chrono::{Duration, NaiveDateTime, Utc};
use std::collections::HashMap;
use std::net::SocketAddr;

#[derive(Default)]
pub struct BanManager {
    pub ips: RwLock<HashMap<SocketAddr, NaiveDateTime>>,
}

//@todo there is a memory leak here. We need to tell the server to run a function every 10 minutes
//(whatever interval doesn't matter). To remove old IPs from this list. If a peer gets banned and
//never connects again, then we will keep that IP in the background. Let's make a function called
//prune() that does this, and then the main app can just spawn it every x interval.
impl BanManager {
    pub fn new() -> Self {
        BanManager {
            ips: RwLock::new(HashMap::new()),
        }
    }

    pub async fn check_banned(&self, addr: &SocketAddr) -> bool {
        let mut to_remove = false;

        let banned = match self.ips.read().await.get(addr) {
            Some(ban_end_time) => {
                if ban_end_time > &Utc::now().naive_utc() {
                    true
                } else {
                    to_remove = true;
                    false
                }
            }
            None => false,
        };

        if to_remove {
            self.remove_ban(addr).await;
        };

        banned
    }

    pub async fn remove_ban(&self, addr: &SocketAddr) {
        self.ips.write().await.remove(addr);
    }

    pub async fn add_ban(&self, addr: &SocketAddr) {
        //1 hour from now - make this a config @todo.
        let ban_time = Utc::now() + Duration::hours(1);

        self.ips.write().await.insert(*addr, ban_time.naive_utc());
    }
}
