use std::{net::IpAddr, sync::Arc, time::Duration};

//@todo wrap this in a Mutex<Arc. Then when a new config is refreshed, everyone can just get it
//from a clone.
#[derive(Default, Clone)]
pub struct ConfigManager {
    config: Arc<Config>,
}

impl ConfigManager {
    pub(crate) fn new(config: Config) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub(crate) fn current_config(&self) -> Arc<Config> {
        self.config.clone()
    }

    // Helpers
    pub(crate) fn proxy_protocol(&self) -> bool {
        self.config.connection.proxy_protocol
    }

    pub(crate) fn default_ban_duration(&self) -> Duration {
        self.config.bans.default_ban_duration
    }
}

#[derive(Clone, Debug, Default)]
pub struct Config {
    pub(crate) connection: ConnectionConfig,
    pub(crate) difficulty: DifficultyConfig,
    pub(crate) bans: BanManagerConfig,
}

#[derive(Clone, Debug)]
pub struct BanManagerConfig {
    pub(crate) default_ban_duration: Duration,
    pub(crate) _ban_score_allowed: u64,
    pub(crate) _whitelisted_ips: Vec<IpAddr>,
    pub(crate) _perma_ban_starting_list: Vec<IpAddr>,
}

impl Default for BanManagerConfig {
    fn default() -> Self {
        BanManagerConfig {
            default_ban_duration: Duration::from_secs(3600),
            _ban_score_allowed: 100,
            _whitelisted_ips: Vec::new(),
            _perma_ban_starting_list: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct ConnectionConfig {
    pub(crate) proxy_protocol: bool,
    pub(crate) max_connections: Option<usize>,
}

#[derive(Clone, Debug, Default)]
pub struct DifficultyConfig {
    pub(crate) initial_difficulty: u64,
    pub(crate) var_diff: bool,
    pub(crate) minimum_difficulty: u64,
    pub(crate) maximum_difficulty: u64,
    //Seconds
    pub(crate) retarget_time: u64,
    //Seconds
    pub(crate) target_time: u64,
    //@todo see if we use this.
    pub(crate) variance_percent: f64,
}

// #[cfg(feature = "upstream")]
// #[derive(Clone, Debug, Default)]
// pub struct UpstreamConfig {
//     pub(crate) enabled: bool,
//     pub(crate) url: String,
// }
