#[cfg(feature = "upstream")]
use crate::config::UpstreamConfig;

use crate::{
    config::VarDiffConfig, id_manager::IDManager, router::Router, types::ReadyIndicator,
    BanManager, ConnectionList, Result, StratumServer,
};
use extended_primitives::Buffer;
use std::{marker::PhantomData, sync::Arc, time::Duration};
use tokio::net::TcpListener;
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
    pub exported_port: Option<u16>,
    pub max_connections: Option<usize>,
    pub proxy: bool,
    pub var_diff_config: VarDiffConfig,
    #[cfg(feature = "upstream")]
    pub upstream_config: UpstreamConfig,
    pub initial_difficulty: u64,
    pub state: State,
    pub connection_state: PhantomData<CState>,
    pub ready_indicator: ReadyIndicator,
    pub shutdown_message: Option<Buffer>,
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    StratumServerBuilder<State, CState>
{
    pub fn new(state: State, server_id: u8) -> Self {
        Self {
            server_id,
            host: String::from(""),
            port: 0,
            #[cfg(feature = "api")]
            api_host: String::from("0.0.0.0"),
            #[cfg(feature = "api")]
            api_port: 8888,
            exported_port: None,
            max_connections: None,
            proxy: false,
            initial_difficulty: 16384,
            state,
            connection_state: PhantomData,
            ready_indicator: ReadyIndicator::new(false),
            var_diff_config: VarDiffConfig {
                var_diff: false,
                minimum_difficulty: 64,
                maximum_difficulty: 4611686018427387904,
                retarget_time: 300,
                target_time: 10,
                variance_percent: 30.0,
            },
            #[cfg(feature = "upstream")]
            upstream_config: UpstreamConfig {
                enabled: false,
                url: String::from(""),
            },
            shutdown_message: None,
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

    #[cfg(feature = "api")]
    pub fn with_api_host(mut self, host: &str) -> Self {
        self.api_host = host.to_owned();
        self
    }

    #[cfg(feature = "api")]
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

    #[cfg(feature = "upstream")]
    pub fn with_upstream(mut self, url: &str) -> Self {
        self.upstream_config = UpstreamConfig {
            enabled: true,
            url: url.to_string(),
        };
        self
    }

    pub fn with_shutdown_message(mut self, msg: Buffer) -> Self {
        self.shutdown_message = Some(msg);
        self
    }

    pub async fn build(self) -> Result<StratumServer<State, CState>> {
        let listener = TcpListener::bind(format!("{}:{}", self.host, self.port)).await?;
        //This will fail if unable to find a local port.
        let listen_address = listener.local_addr()?;
        let connection_list = Arc::new(ConnectionList::new(self.max_connections));
        let listener = TcpListenerStream::new(listener);

        //@todo we should probably just allow this to be Option::None>
        let expected_port = match self.exported_port {
            Some(exported_port) => exported_port,
            None => self.port,
        };

        let cancel_token = CancellationToken::new();

        let ban_manager = Arc::new(BanManager::new(
            cancel_token.child_token(),
            // 1 Hour, @todo come from settings
            Duration::from_secs(60 * 60),
        ));

        #[cfg(feature = "api")]
        let api = {
            let state = crate::api::Context {
                ban_manager: ban_manager.clone(),
                ready_indicator: self.ready_indicator.create_new(),
            };

            let api_address = format!("{}:{}", self.api_host, self.api_port).parse()?;

            //@todo also pass in a port for metrics
            crate::api::Api::build(api_address, state)?
        };

        Ok(StratumServer {
            id: self.server_id,
            listener,
            listen_address,
            expected_port,
            proxy: self.proxy,
            initial_difficulty: self.initial_difficulty,
            connection_list,
            state: self.state,
            ban_manager,
            router: Arc::new(Router::new()),
            var_diff_config: self.var_diff_config,
            session_id_manager: Arc::new(IDManager::new(self.server_id)),
            cancel_token,
            global_thread_list: Vec::new(),
            ready_indicator: self.ready_indicator,
            shutdown_message: self.shutdown_message,

            // #[cfg(feature = "api")]
            // api: Arc::new(Mutex::new(api)),
            #[cfg(feature = "api")]
            api,
            #[cfg(feature = "upstream")]
            upstream_router: Arc::new(Router::new()),
            #[cfg(feature = "upstream")]
            upstream_config: self.upstream_config,
        })
    }
}
