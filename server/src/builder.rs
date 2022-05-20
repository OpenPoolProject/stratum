use crate::{
    config::{UpstreamConfig, VarDiffConfig},
    id_manager::IDManager,
    router::Router,
    types::ReadyIndicator,
    BanManager, ConnectionList, StratumServer,
};
use async_std::sync::{Arc, Mutex};
use std::marker::PhantomData;
use stop_token::StopSource;

#[derive(Default)]
pub struct StratumServerBuilder<State, CState> {
    pub server_id: u8,
    pub host: String,
    pub port: u16,
    pub api_host: String,
    pub api_port: u16,
    pub exported_port: Option<u16>,
    pub max_connections: Option<usize>,
    pub proxy: bool,
    pub var_diff_config: VarDiffConfig,
    pub upstream_config: UpstreamConfig,
    pub initial_difficulty: u64,
    pub state: State,
    pub connection_state: PhantomData<CState>,
    pub ready_indicator: ReadyIndicator,
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    StratumServerBuilder<State, CState>
{
    pub fn new(state: State, server_id: u8) -> Self {
        Self {
            server_id,
            host: String::from(""),
            port: 0,
            api_host: String::from("0.0.0.0"),
            api_port: 8888,
            exported_port: None,
            max_connections: None,
            proxy: false,
            initial_difficulty: 16384,
            state,
            connection_state: PhantomData,
            ready_indicator: ReadyIndicator::new(true),
            var_diff_config: VarDiffConfig {
                var_diff: false,
                minimum_difficulty: 64,
                maximum_difficulty: 4611686018427387904,
                retarget_time: 300,
                target_time: 10,
                variance_percent: 30.0,
            },
            upstream_config: UpstreamConfig {
                enabled: false,
                url: String::from(""),
            },
        }
    }

    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.to_owned();
        self
    }

    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn with_api_host(mut self, host: &str) -> Self {
        self.api_host = host.to_owned();
        self
    }

    pub fn with_api_port(mut self, port: u16) -> Self {
        self.api_port = port;
        self
    }

    pub fn with_max_connections(mut self, max_connections: usize) -> Self {
        self.max_connections = Some(max_connections);
        self
    }

    pub fn with_proxy(mut self, value: bool) -> Self {
        self.proxy = value;
        self
    }

    pub fn with_var_diff(mut self, value: bool) -> Self {
        self.var_diff_config.var_diff = value;
        self
    }

    pub fn with_minimum_difficulty(mut self, difficulty: u64) -> Self {
        self.var_diff_config.minimum_difficulty = difficulty;
        self
    }

    pub fn with_maximum_difficulty(mut self, difficulty: u64) -> Self {
        self.var_diff_config.maximum_difficulty = difficulty;
        self
    }

    pub fn with_retarget_time(mut self, time: u64) -> Self {
        self.var_diff_config.retarget_time = time;
        self
    }

    pub fn with_target_time(mut self, time: u64) -> Self {
        self.var_diff_config.target_time = time;
        self
    }

    pub fn with_variance_percent(mut self, percent: f64) -> Self {
        self.var_diff_config.variance_percent = percent;
        self
    }

    pub fn with_initial_difficulty(mut self, difficulty: u64) -> Self {
        self.initial_difficulty = difficulty;
        self
    }

    pub fn with_expected_port(mut self, port: u16) -> Self {
        self.exported_port = Some(port);
        self
    }

    pub fn with_ready_indicator(mut self, ready_indicator: ReadyIndicator) -> Self {
        self.ready_indicator = ready_indicator;
        self
    }

    pub fn with_upstream(mut self, url: &str) -> Self {
        self.upstream_config = UpstreamConfig {
            enabled: true,
            url: url.to_string(),
        };
        self
    }

    pub fn build(self) -> StratumServer<State, CState> {
        let connection_list = Arc::new(ConnectionList::new(self.max_connections));

        let expected_port = match self.exported_port {
            Some(exported_port) => exported_port,
            None => self.port,
        };

        let stop_source = StopSource::new();
        let stop_token = stop_source.token();

        StratumServer {
            id: self.server_id,
            host: self.host,
            port: self.port,
            expected_port,
            proxy: self.proxy,
            initial_difficulty: self.initial_difficulty,
            connection_list,
            state: self.state,
            ban_manager: Arc::new(BanManager::new()),
            router: Arc::new(Router::new()),
            upstream_router: Arc::new(Router::new()),
            var_diff_config: self.var_diff_config,
            upstream_config: self.upstream_config,
            session_id_manager: Arc::new(IDManager::new(self.server_id)),
            stop_source: Arc::new(Mutex::new(Some(stop_source))),
            stop_token,
            global_thread_list: Vec::new(),
            ready_indicator: self.ready_indicator,
        }
    }
}
