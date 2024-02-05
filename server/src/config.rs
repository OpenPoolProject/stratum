use std::{net::IpAddr, sync::Arc, time::Duration};

//@todo wrap this in a Mutex<Arc. Then when a new config is refreshed, everyone can just get it
//from a clone.
#[derive(Default, Clone, Debug)]
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

    pub(crate) fn difficulty_config(&self) -> &DifficultyConfig {
        &self.config.difficulty
    }

    pub(crate) fn connection_config(&self) -> &ConnectionConfig {
        &self.config.connection
    }

    pub(crate) fn ban_manager_enabled(&self) -> bool {
        self.config.bans.enabled
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
    pub(crate) enabled: bool,
    pub(crate) default_ban_duration: Duration,
    pub(crate) _ban_score_allowed: u64,
    pub(crate) _whitelisted_ips: Vec<IpAddr>,
    pub(crate) _perma_ban_starting_list: Vec<IpAddr>,
}

impl Default for BanManagerConfig {
    fn default() -> Self {
        BanManagerConfig {
            enabled: false,
            default_ban_duration: Duration::from_secs(3600),
            _ban_score_allowed: 100,
            _whitelisted_ips: Vec::new(),
            _perma_ban_starting_list: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ConnectionConfig {
    pub(crate) proxy_protocol: bool,
    pub(crate) max_connections: Option<usize>,
    /// Active Timeout is how long with no activity before we disconnect a miner.
    pub(crate) active_timeout: u64,
    /// Initial Timeout is how long we wait for an initial message from a miner. Once they are
    /// active, we switch to the active timeout setting.
    pub(crate) inital_timeout: u64,
    //@todo maybe move this to a new struct called MinerConfig, but for now I think it's ok.
    /// Check Threshold is how many shares until we consider a ban on a miner
    pub(crate) check_threshold: u64,
    /// Invalid Percent is the percent of shares that are rejected or stale before we ban a miner.
    /// In full-interval format e.g. 50.0 = 50%.
    pub(crate) invalid_percent: f64,
}

impl Default for ConnectionConfig {
    fn default() -> Self {
        ConnectionConfig {
            proxy_protocol: false,
            max_connections: None,
            active_timeout: 600,
            inital_timeout: 15,
            check_threshold: 500,
            invalid_percent: 50.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DifficultyConfig {
    pub(crate) retarget_share_amount: u64,
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

impl Default for DifficultyConfig {
    fn default() -> Self {
        DifficultyConfig {
            retarget_share_amount: 30,
            initial_difficulty: 16384,
            var_diff: false,
            minimum_difficulty: 64,
            maximum_difficulty: 4_611_686_018_427_387_904,
            retarget_time: 300,
            target_time: 10,
            variance_percent: 30.0,
        }
    }
}

impl DifficultyConfig {
    pub(crate) fn initial_retarget_time(&self, now: u128) -> u128 {
        now - ((self.retarget_time as u128 * 1000) / 2)
    }
}

// #[cfg(feature = "upstream")]
// #[derive(Clone, Debug, Default)]
// pub struct UpstreamConfig {
//     pub(crate) enabled: bool,
//     pub(crate) url: String,
// }
