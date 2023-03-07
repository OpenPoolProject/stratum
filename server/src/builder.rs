use crate::{
    config::{ConnectionConfig, DifficultyConfig},
    id_manager::IDManager,
    router::Router,
    types::ReadyIndicator,
    BanManager, Config, ConfigManager, Result, SessionList, StratumServer,
};
use extended_primitives::Buffer;
use std::{marker::PhantomData, sync::Arc};
use tokio::{net::TcpListener, task::JoinSet};
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct StratumServerBuilder<State, CState> {
    pub server_id: u8,
    pub host: String,
    pub port: u16,
    #[cfg(feature = "api")]
    pub api_host: String,
    #[cfg(feature = "api")]
    pub api_port: u16,
    pub connection_config: ConnectionConfig,
    pub var_diff_config: DifficultyConfig,
    pub state: State,
    pub connection_state: PhantomData<CState>,
    pub ready_indicator: ReadyIndicator,
    pub shutdown_message: Option<Buffer>,
    pub cancel_token: Option<CancellationToken>,
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    StratumServerBuilder<State, CState>
{
    pub fn new(state: State, server_id: u8) -> Self {
        Self {
            server_id,
            host: String::new(),
            port: 0,
            #[cfg(feature = "api")]
            api_host: String::from("0.0.0.0"),
            #[cfg(feature = "api")]
            api_port: 8888,
            connection_config: ConnectionConfig::default(),
            state,
            connection_state: PhantomData,
            ready_indicator: ReadyIndicator::new(false),
            var_diff_config: DifficultyConfig {
                retarget_share_amount: 30,
                initial_difficulty: 16384,
                var_diff: false,
                minimum_difficulty: 64,
                maximum_difficulty: 4_611_686_018_427_387_904,
                retarget_time: 300,
                target_time: 10,
                variance_percent: 30.0,
            },
            // #[cfg(feature = "upstream")]
            // upstream_config: UpstreamConfig {
            //     enabled: false,
            //     url: String::from(""),
            // },
            shutdown_message: None,
            cancel_token: None,
        }
    }

    #[must_use]
    pub fn with_host(mut self, host: &str) -> Self {
        self.host = host.to_owned();
        self
    }

    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    #[cfg(feature = "api")]
    #[must_use]
    pub fn with_api_host(mut self, host: &str) -> Self {
        self.api_host = host.to_owned();
        self
    }

    #[cfg(feature = "api")]
    #[must_use]
    pub fn with_api_port(mut self, port: u16) -> Self {
        self.api_port = port;
        self
    }

    #[must_use]
    pub fn with_max_connections(mut self, max_connections: usize) -> Self {
        self.connection_config.max_connections = Some(max_connections);
        self
    }

    #[must_use]
    pub fn with_proxy(mut self, value: bool) -> Self {
        self.connection_config.proxy_protocol = value;
        self
    }

    #[must_use]
    pub fn with_var_diff(mut self, value: bool) -> Self {
        self.var_diff_config.var_diff = value;
        self
    }

    #[must_use]
    pub fn with_minimum_difficulty(mut self, difficulty: u64) -> Self {
        self.var_diff_config.minimum_difficulty = difficulty;
        self
    }

    #[must_use]
    pub fn with_maximum_difficulty(mut self, difficulty: u64) -> Self {
        self.var_diff_config.maximum_difficulty = difficulty;
        self
    }

    #[must_use]
    pub fn with_retarget_time(mut self, time: u64) -> Self {
        self.var_diff_config.retarget_time = time;
        self
    }

    #[must_use]
    pub fn with_target_time(mut self, time: u64) -> Self {
        self.var_diff_config.target_time = time;
        self
    }

    #[must_use]
    pub fn with_variance_percent(mut self, percent: f64) -> Self {
        self.var_diff_config.variance_percent = percent;
        self
    }

    #[must_use]
    pub fn with_initial_difficulty(mut self, difficulty: u64) -> Self {
        self.var_diff_config.initial_difficulty = difficulty;
        self
    }

    #[must_use]
    pub fn with_ready_indicator(mut self, ready_indicator: ReadyIndicator) -> Self {
        self.ready_indicator = ready_indicator;
        self
    }

    #[must_use]
    pub fn with_shutdown_message(mut self, msg: Buffer) -> Self {
        self.shutdown_message = Some(msg);
        self
    }

    #[must_use]
    pub fn with_cancel_token(mut self, token: CancellationToken) -> Self {
        self.cancel_token = Some(token);
        self
    }

    pub async fn build(self) -> Result<StratumServer<State, CState>> {
        let config = Config {
            connection: self.connection_config,
            difficulty: self.var_diff_config,
            bans: crate::config::BanManagerConfig::default(),
        };

        let config_manager = ConfigManager::new(config);

        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).await?;
        //This will fail if unable to find a local port.
        let listen_address = listener.local_addr()?;
        let listener = TcpListenerStream::new(listener);
        let session_list = SessionList::new(config_manager.clone());

        let cancel_token = if let Some(cancel_token) = self.cancel_token {
            cancel_token
        } else {
            CancellationToken::new()
        };

        let ban_manager = BanManager::new(config_manager.clone(), cancel_token.child_token());

        #[cfg(feature = "api")]
        let api = {
            let state = crate::api::Context {
                ban_manager: ban_manager.clone(),
                ready_indicator: self.ready_indicator.create_new(),
            };

            let api_address = format!("{}:{}", self.api_host, self.api_port).parse()?;

            crate::api::Api::build(api_address, state)?
        };

        Ok(StratumServer {
            id: self.server_id,
            listener,
            listen_address,
            session_list,
            config_manager,
            state: self.state,
            ban_manager,
            router: Arc::new(Router::new()),
            session_id_manager: IDManager::new(self.server_id),
            cancel_token,
            global_thread_list: JoinSet::new(),
            ready_indicator: self.ready_indicator,
            shutdown_message: self.shutdown_message,
            #[cfg(feature = "api")]
            api,
        })
    }
}
