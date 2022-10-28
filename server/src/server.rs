#[cfg(feature = "upstream")]
use crate::UpstreamConfig;

use crate::{
    global::Global,
    route::Endpoint,
    router::Router,
    types::{GlobalVars, ReadyIndicator},
    BanManager, ConnectionList, Result, StratumServerBuilder, VarDiffConfig,
};
use futures::StreamExt;
use std::sync::Arc;
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_stream::wrappers::TcpListenerStream;
use tracing::info;
// use metrics_exporter_prometheus::PrometheusBuilder;
use signal_hook::consts::signal::*;
use signal_hook_tokio::{Handle, Signals};
//@todo maybe remove this with time dependency
use extended_primitives::Buffer;
use std::time::Instant;
use tokio_util::sync::CancellationToken;

//@todo make this not default, but api.
// #[cfg(feature = "default")]
// use crate::api::init_api_server;

// use crate::metrics::Metrics;

use crate::id_manager::IDManager;

use crate::tcp::handle_connection;

// #[derive(Clone)]
pub struct StratumServer<State, CState>
where
    State: Clone + Send + Sync + 'static,
    CState: Default + Clone + Send + Sync + 'static,
{
    pub(crate) id: u8,
    pub(crate) host: String,
    pub(crate) port: u16,
    pub(crate) expected_port: u16,
    pub(crate) proxy: bool,
    pub(crate) initial_difficulty: u64,
    pub(crate) state: State,
    pub(crate) connection_list: Arc<ConnectionList<CState>>,
    pub(crate) ban_manager: Arc<BanManager>,
    pub(crate) router: Arc<Router<State, CState>>,
    pub(crate) var_diff_config: VarDiffConfig,
    #[cfg(feature = "upstream")]
    pub(crate) upstream_router: Arc<Router<State, CState>>,
    #[cfg(feature = "upstream")]
    pub(crate) upstream_config: UpstreamConfig,
    pub(crate) session_id_manager: Arc<IDManager>,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) global_thread_list: Vec<JoinHandle<()>>,
    //@todo I think we can actually kill this now that I remember.
    //Likely why the nimiq server will occasionally get not ready for a long time.
    //Although let's look into what scenarios are we marking not ready in the stratums.
    //@todo I think revamp this a bit to include getting the correct server_id from our
    //homebase
    pub(crate) ready_indicator: ReadyIndicator,
    pub(crate) shutdown_message: Option<Buffer>,
}

//@todo consider this signal for reloading upstream config?
// SIGHUP => {
//     // Reload configuration
//     // Reopen the log file
// }
//@todo maybe move this somewhere else outside of this file.
async fn handle_signals(mut signals: Signals, cancel_token: CancellationToken) {
    while let Some(signal) = signals.next().await {
        tracing::warn!("{:?}", &signal);
        match signal {
            SIGTERM | SIGINT | SIGQUIT => {
                // Shutdown the system;
                //@todo print the signal here
                //@todo use extended signal information
                info!("Received SIGINT. Initiating shutdown...");
                cancel_token.cancel();
                info!("CancellationToken has been canceled");
                return;
            }
            _ => unreachable!(),
        }
    }
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    StratumServer<State, CState>
{
    pub fn builder(state: State, server_id: u8) -> StratumServerBuilder<State, CState> {
        StratumServerBuilder::new(state, server_id)
    }

    //Initialize the server before we want to start accepting any connections.
    async fn init(&self) -> Result<(Handle, JoinHandle<()>)> {
        info!("Initializing...");

        // if cfg!(feature = "default") {
        //     init_api_server(self.stop_token.clone(), self.ready_indicator.clone()).await?;
        //     info!("API Server Initialized");
        // }

        let signals = Signals::new([SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
        let handle = signals.handle();

        //@note we want to clone here because cancel will cancel all child tokens. If we set this
        //as a child token, it will have no children itself
        let signals_task = tokio::task::spawn(handle_signals(signals, self.cancel_token.clone()));

        info!("Initialization Complete");

        Ok((handle, signals_task))
    }

    pub fn add(&mut self, method: &str, ep: impl Endpoint<State, CState>) {
        //@todo review this code.
        let router = Arc::get_mut(&mut self.router)
            .expect("Registering routes is not possible after the Server has started");
        router.add(method, ep);
    }

    //@todo will probably change this "Endpoint" here to an upstream endpoint.
    #[cfg(feature = "upstream")]
    pub fn add_upstream(&mut self, method: &str, ep: impl Endpoint<State, CState>) {
        //@todo review this code.
        let router = Arc::get_mut(&mut self.upstream_router)
            .expect("Registering routes is not possible after the Server has started");
        router.add(method, ep);
    }

    pub fn global(&mut self, global_name: &str, ep: impl Global<State, CState>) {
        let state = self.state.clone();
        let connection_list = self.connection_list.clone();
        let cancel_token = self.get_cancel_token();

        let handle = tokio::spawn(async move {
            tokio::select! {
                res = ep.call(state, connection_list) => {
                    //@todo call does not return an Error. It should!
                    // if let Err(e) = res {
                    //     //@todo more indepth, lots of stuff to include here.
                    //     error!("Global thread failed.");
                    // }
                }
                _ = cancel_token.cancelled() => {
                    //@todo
                    info!("Global thread XYZ is shutting down from shutdown message.");
                }

            }
            // let call = ep.call(state, connection_list).timeout_at(stop_token);
            // match call.await {
            //     Ok(()) => {}
            //     Err(_e) => {
            //         //@todo we can't do any of this until Tokio stablizes these APIs
            //         //@todo - This will only be relevant for when we have results returned by
            //         //Globals because currently this block will only be called if the stop
            //         //token is revoked. Re-enable this when that is implemented.
            //         //2. We should probably have a config setting that asks if we want to nuke everything if a
            //         //   global falls. I don't know if we should automatically nuke everything, but it should
            //         //   be a setting that is available. Maybe it's available on a per-global basis, but I
            //         //   think it's something we should absolutely know about.
            //         //   @
            //         // warn!(
            //         //     "Global thread id: {} name: {} was unexpected closed by the error: {}",
            //         //     async_std::task::current().id(),
            //         //     async_std::task::current().name().unwrap_or(""),
            //         //     e
            //         // );
            //         // info!(
            //         //     "Global thread id: {} name: {} was closed",
            //         //     async_std::task::current().id(),
            //         //     async_std::task::current().name().unwrap_or("")
            //         // );
            //     }
            // }
        });

        // let handle = async_std::task::Builder::new()
        //     .name(global_name.to_string())
        //     .spawn(async move {
        //         let call = ep.call(state, connection_list).timeout_at(stop_token);
        //         match call.await {
        //             Ok(()) => {}
        //             Err(_e) => {
        //                 //@todo - This will only be relevant for when we have results returned by
        //                 //Globals because currently this block will only be called if the stop
        //                 //token is revoked. Re-enable this when that is implemented.
        //                 //2. We should probably have a config setting that asks if we want to nuke everything if a
        //                 //   global falls. I don't know if we should automatically nuke everything, but it should
        //                 //   be a setting that is available. Maybe it's available on a per-global basis, but I
        //                 //   think it's something we should absolutely know about.
        //                 //   @
        //                 // warn!(
        //                 //     "Global thread id: {} name: {} was unexpected closed by the error: {}",
        //                 //     async_std::task::current().id(),
        //                 //     async_std::task::current().name().unwrap_or(""),
        //                 //     e
        //                 // );
        //                 info!(
        //                     "Global thread id: {} name: {} was closed",
        //                     async_std::task::current().id(),
        //                     async_std::task::current().name().unwrap_or("")
        //                 );
        //             }
        //         };
        //     })
        //     //@todo switch this to expect
        //     .unwrap();

        self.global_thread_list.push(handle);
    }

    async fn handle_incoming(&mut self) -> Result<()> {
        let listen_url = format!("{}:{}", &self.host, self.port);

        let listener = TcpListener::bind(&listen_url).await?;

        let mut incoming = TcpListenerStream::new(listener);

        info!("Listening on {}", &listen_url);

        while let Some(stream) = incoming.next().await {
            let stream = match stream {
                Ok(stream) => stream,
                //@todo this needs to be handled a lot better, but for now I hope this solves our
                //issue.
                Err(_e) => continue,
            };

            let addr = match stream.peer_addr() {
                Ok(addr) => addr,
                Err(_e) => continue,
            };

            //@todo a lot of these clones can be moved into the async block (or rather, just
            //before). Might clean this section up a bit.
            //Wrap this in an if proxy
            let connection_list = self.connection_list.clone();
            let id_manager = self.session_id_manager.clone();
            let proxy = self.proxy;
            let expected_port = self.expected_port;
            let initial_difficulty = self.initial_difficulty;
            let ban_manager = self.ban_manager.clone();
            let router = self.router.clone();
            #[cfg(feature = "upstream")]
            let upstream_router = self.upstream_router.clone();
            let state = self.state.clone();
            let connection_state = CState::default();

            let var_diff_config = self.var_diff_config.clone();
            #[cfg(feature = "upstream")]
            let upstream_config = self.upstream_config.clone();
            let global_vars = GlobalVars::new(self.id);

            //@todo should we pass the stop token in this?
            let handle = tokio::task::spawn(async move {
                //First things first, since we get a ridiculous amount of no-data stream connections, we are
                //going to peak 2 bytes off of this and if we get those 2 bytes we continue, otherwise we are
                //burning it down with 0 logs baby.
                //@todo for safety reasons we may want to make this like 5 bytes, but thats a rare
                //scenario so I'm not too pressed for it imo.
                let mut check_bytes = [0u8; 4];
                match stream.peek(&mut check_bytes).await {
                    Err(e) => {
                        tracing::error!(
                            "Trouble reading peak bytes from stream: {}. Exiting. Error: {}",
                            addr,
                            e
                        );
                        return;
                    }
                    Ok(n) => {
                        if n == 0 {
                            //@todo decide if we want to log this at all, but I'd say probably not.
                            return;
                        } else if n >= 4 {
                            //Gucci
                        }
                        // else {
                        // Right now we do nothing for this block, but catching small byte attacks
                        // here might be very helpful for us in the long run.
                        //     //
                        // }
                    }
                };

                handle_connection(
                    id_manager,
                    ban_manager,
                    addr,
                    connection_list,
                    router,
                    #[cfg(feature = "upstream")]
                    upstream_router,
                    #[cfg(feature = "upstream")]
                    upstream_config,
                    state,
                    stream,
                    var_diff_config,
                    initial_difficulty,
                    connection_state,
                    //@todo would be nice to be Proxy Config.
                    //@TODO DO THIS TONIGHT
                    proxy,
                    expected_port,
                    global_vars,
                )
                //@todo think about it
                .await
                .unwrap();
            });

            //@todo this has got to be a memory leak no?
            //@TODO really need to fix this.
            //https://github.com/smol-rs/async-task/pull/19
            //@todo also see this: https://web.mit.edu/rust-lang_v1.25/arch/amd64_ubuntu1404/share/doc/rust/html/book/second-edition/ch20-06-graceful-shutdown-and-cleanup.html
            // thread_list.push(handle);
        }

        Ok(())
    }

    pub async fn start(&mut self) -> Result<()> {
        //Initalize the recorder
        init_metrics_recorder();

        let (signal_handle, signal_task) = self.init().await?;

        let cancel_token = self.cancel_token.clone();

        tokio::select! {
            _ = self.handle_incoming() => {
                //@todo I think we want to handle errors here.
                //This would be the listening erroring out.
            },
            _ = cancel_token.cancelled() => {}
        }
        //@todo removing this to see if it's the cause of the memory leak. Would be ince to write
        //some tests to see if this causes any leaked threads.
        // let mut thread_list = Vec::new();
        // let mut incoming_with_stop = incoming.timeout_at(self.stop_token.clone());

        let start = Instant::now();

        //@todo might be nice to have self.shutdown here?

        //Before we return Ok here, we need to finish cleaning up the rest.
        //So what I'm thinking we do is iterate through miner list and shutdown everything.
        //@todo magic number of 60 should be set somewhere
        self.connection_list
            .shutdown(self.shutdown_message.clone(), 60)
            .await?;

        //I believe that shutdown here should basically trigger connect_list to self distruct.
        //That being said, if each handle_connection uses the same stop_token, then it won't even
        //be necessary to do this. We can just iterate through the futures (task_handles), and
        //await them all.
        //
        //Okay so I'm thinking one way to possibly do this is store an Option<StopSource> in
        //connection....
        //
        //Do the same for globals.

        let global_thread_list = self.global_thread_list.drain(..);

        //@TODO make this parrallel.
        info!("Awaiting for all current globals to complete");
        for thread in global_thread_list {
            thread.await;
        }

        //@TODO make this parrallel.
        // info!("Awaiting for all current connections to complete");
        // for thread in thread_list {
        //     thread.await;
        // }

        // Allow for signal threads to close. @todo check this in testing.
        info!("Awaiting for all signal handler to complete");
        signal_handle.close();
        info!("Awaiting for all signal task to complete");
        signal_task.await;

        //@todo lets get some better parsing here. Seconds and NS would be great
        info!("Shutdown complete in {} ns", start.elapsed().as_nanos());

        Ok(())
    }

    pub fn get_ready_indicator(&self) -> ReadyIndicator {
        self.ready_indicator.clone()
    }

    // #[cfg(test)]
    pub fn get_miner_list(&self) -> Arc<ConnectionList<CState>> {
        self.connection_list.clone()
    }

    pub fn get_cancel_token(&self) -> CancellationToken {
        self.cancel_token.child_token()
    }
}

//Initalizes the prometheus metrics recorder.
pub fn init_metrics_recorder() {
    // let (recorder, _) = PrometheusBuilder::new().build().unwrap();

    ////@todo this is breaking.
    // metrics::set_boxed_recorder(Box::new(recorder));
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static> Drop
    for StratumServer<State, CState>
{
    fn drop(&mut self) {
        info!("Dropping StratumSever with data `{}`!", self.host);
    }
}
