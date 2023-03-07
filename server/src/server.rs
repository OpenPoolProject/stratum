use crate::{
    global::Global,
    id_manager::IDManager,
    route::Endpoint,
    router::Router,
    tcp::Handler,
    types::{ConnectionID, GlobalVars, ReadyIndicator},
    BanManager, ConfigManager, Connection, Result, SessionList, StratumServerBuilder,
};
use extended_primitives::Buffer;
use futures::StreamExt;
use rlimit::Resource;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::task::JoinSet;
use tokio_stream::wrappers::TcpListenerStream;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, trace, warn};

pub struct StratumServer<State, CState>
where
    State: Clone,
    CState: Default + Clone,
{
    pub(crate) id: u8,
    pub(crate) listen_address: SocketAddr,
    pub(crate) listener: TcpListenerStream,
    pub(crate) state: State,
    pub(crate) session_list: SessionList<CState>,
    pub(crate) ban_manager: BanManager,
    pub(crate) config_manager: ConfigManager,
    pub(crate) router: Arc<Router<State, CState>>,
    pub(crate) session_id_manager: IDManager,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) global_thread_list: JoinSet<()>,
    pub(crate) ready_indicator: ReadyIndicator,
    pub(crate) shutdown_message: Option<Buffer>,
    #[cfg(feature = "api")]
    pub(crate) api: crate::api::Api,
}

impl<State, CState> StratumServer<State, CState>
where
    State: Clone + Send + Sync + 'static,
    CState: Default + Clone + Send + Sync + 'static,
{
    pub fn builder(state: State, server_id: u8) -> StratumServerBuilder<State, CState> {
        StratumServerBuilder::new(state, server_id)
    }

    pub fn add(&mut self, method: &str, ep: impl Endpoint<State, CState>) {
        let router = Arc::get_mut(&mut self.router)
            .expect("Registering routes is not possible after the Server has started");
        router.add(method, ep);
    }

    pub fn global(&mut self, global_name: &str, ep: impl Global<State, CState>) {
        self.global_thread_list.spawn({
            let state = self.state.clone();
            let session_list = self.session_list.clone();
            let cancel_token = self.get_cancel_token();
            let global_name = global_name.to_string();
            async move {
                tokio::select! {
                    res = ep.call(state, session_list) => {
                        if let Err(e) = res {
                            error!(cause = ?e, "Global thread {} failed.", global_name);
                        }
                    }
                    _ = cancel_token.cancelled() => {
                        info!("Global thread {} is shutting down from shutdown message.", global_name);
                    }

                }
            }
        });
    }

    async fn handle_incoming(&mut self) -> Result<()> {
        info!("Listening on {}", &self.listen_address);

        while let Some(stream) = self.listener.next().await {
            let stream = match stream {
                Ok(stream) => stream,
                Err(e) => {
                    error!(cause = ?e, "Unable to access stream");
                    continue;
                }
            };

            let id = ConnectionID::new();
            let child_token = self.get_cancel_token();

            trace!(
                id = ?id,
                ip = &stream.peer_addr()?.to_string(),
                "Connection initialized",
            );

            let connection = match Connection::new(id.clone(), stream, child_token.clone()) {
                Ok(connection) => connection,
                Err(e) => {
                    error!(id = ?id, cause = ?e, "Failed while constructing Connection");
                    continue;
                }
            };

            let handler = Handler {
                id: id.clone(),
                ban_manager: self.ban_manager.clone(),
                id_manager: self.session_id_manager.clone(),
                session_list: self.session_list.clone(),
                router: self.router.clone(),
                state: self.state.clone(),
                connection_state: CState::default(),
                config_manager: self.config_manager.clone(),
                cancel_token: child_token,
                global_vars: GlobalVars::new(self.id),
                connection,
            };

            tokio::spawn(async move {
                if let Err(err) = handler.run().await {
                    error!(id =?id, cause = ?err, "connection error");
                }
            });
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        init()?;

        let cancel_token = self.cancel_token.clone();

        #[cfg(feature = "api")]
        let api_handle = self.api.run(cancel_token.clone())?;

        tokio::select! {
            res = self.handle_incoming() => {
                if let Err(err) = res {
                    error!(cause = %err, "failed to accept");
                };
            },
            _ = cancel_token.cancelled() => {}
        }

        let start = Instant::now();

        //Session Shutdowns
        {
            self.session_list
                .shutdown_msg(self.shutdown_message.clone())?;

            let mut backoff = 1;
            loop {
                let connected_miners = self.session_list.len();
                if connected_miners == 0 {
                    break;
                }

                if backoff > 64 {
                    warn!("{connected_miners} remaining, force shutting down now");
                    self.session_list.shutdown();
                    break;
                }

                info!("Waiting for all miners to disconnect, {connected_miners} remaining");
                tokio::time::sleep(Duration::from_secs(backoff)).await;

                backoff *= 2;
            }
        }

        info!("Awaiting for all current globals to complete");
        while let Some(res) = self.global_thread_list.join_next().await {
            if let Err(err) = res {
                error!(cause = %err, "Global thread failed to shut down gracefully.");
            }
        }

        #[cfg(feature = "api")]
        {
            info!("Waiting for Api handler to finish");
            if let Err(err) = api_handle.await {
                error!(cause = %err, "API failed to shut down gracefully.");
            }
        }

        info!("Shutdown complete in {} ns", start.elapsed().as_nanos());

        Ok(())
    }

    pub fn get_ready_indicator(&self) -> ReadyIndicator {
        self.ready_indicator.create_new()
    }

    // #[cfg(test)]
    pub fn get_miner_list(&self) -> SessionList<CState> {
        self.session_list.clone()
    }

    pub fn get_cancel_token(&self) -> CancellationToken {
        self.cancel_token.child_token()
    }

    pub fn get_address(&self) -> SocketAddr {
        self.listen_address
    }

    pub fn get_ban_manager(&self) -> BanManager {
        self.ban_manager.clone()
    }

    #[cfg(feature = "api")]
    pub fn get_api_address(&self) -> SocketAddr {
        self.api.listen_address()
    }
}

fn init() -> Result<()> {
    info!("Initializing...");

    //Check that the system will support what we need.
    let (hard, soft) = rlimit::getrlimit(Resource::NOFILE)?;

    info!("Current Ulimit is set to {hard} hard limit, {soft} soft limit");

    info!("Initialization Complete");

    Ok(())
}

//@todo
// #[cfg(test)]
// mod tests {
//
//     #[derive(Clone)]
//     pub struct State {}
//
//     pub async fn handle_auth(
//         req: StratumRequest<State>,
//         _connection: Arc<Session<ConnectionState>>,
//     ) -> Result<bool> {
//         let state = req.state();
//
//         let login = state.auth.login().await;
//
//         Ok(login)
//     }
//
//     #[tokio::test]
//     async fn test_server_add() {
//         let builder = StratumServer::builder(state, 1)
//             .with_host("0.0.0.0")
//             .with_port(0);
//
//         let mut server = builder.build().await?;
//
//         let address = server.get_address();
//
//         server.add("auth", handle_auth);
//     }
// }
