use crate::{
    global::Global,
    route::Endpoint,
    router::Router,
    types::{GlobalVars, ReadyIndicator},
    BanManager, ConnectionList, Result, StratumServerBuilder, UpstreamConfig, VarDiffConfig,
};
use async_std::{
    net::TcpListener,
    sync::{Arc, Mutex},
    task,
    task::JoinHandle,
};
use futures::{channel::mpsc::unbounded, io::AsyncReadExt, SinkExt, StreamExt};
use log::{info, warn};
// use metrics_exporter_prometheus::PrometheusBuilder;
use smol::Async;
use std::{os::unix::net::UnixStream, time::Instant};
use stop_token::{future::FutureExt, stream::StreamExt as StopStreamExt, StopSource, StopToken};

//@todo make this not default, but api.
#[cfg(feature = "default")]
use crate::api::init_api_server;

// use crate::metrics::Metrics;

use crate::id_manager::IDManager;
#[cfg(not(feature = "websockets"))]
use crate::tcp::handle_connection;

#[cfg(feature = "websockets")]
use crate::websockets::handle_connection;

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
    pub(crate) upstream_router: Arc<Router<State, CState>>,
    pub(crate) var_diff_config: VarDiffConfig,
    pub(crate) upstream_config: UpstreamConfig,
    pub(crate) session_id_manager: Arc<IDManager>,
    //@todo maybe wrap this in something more friendly... Can use it in connection as well.
    pub(crate) stop_source: Arc<Mutex<Option<StopSource>>>,
    pub(crate) stop_token: StopToken,
    pub(crate) global_thread_list: Vec<JoinHandle<()>>,
    //@todo I think we can actually kill this now that I remember.
    //Likely why the nimiq server will occasionally get not ready for a long time.
    //Although let's look into what scenarios are we marking not ready in the stratums.
    //@todo I think revamp this a bit to include getting the correct server_id from our
    //homebase
    pub(crate) ready_indicator: ReadyIndicator,
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    StratumServer<State, CState>
{
    pub fn builder(state: State, server_id: u8) -> StratumServerBuilder<State, CState> {
        StratumServerBuilder::new(state, server_id)
    }

    //Initialize the server before we want to start accepting any connections.
    async fn init(&self) -> Result<()> {
        info!("Initializing...");

        // let stop_source = StopSource::new();

        if cfg!(feature = "default") {
            init_api_server(self.stop_token.clone(), self.ready_indicator.clone()).await?;
            info!("API Server Initialized");
        }

        //This is the shutdown channel. If we get a result from here, then we shutdown.
        let (tx, mut rx) = unbounded();
        let tx = Arc::new(Mutex::new(tx));

        //Handle signals here.
        //@todo probably move this code elsewhere as it's a bit messy -> And could use some
        //cleanups.
        //@todo also figure out to test this BOTH signterm and sigint
        let (a, mut b) = Async::<UnixStream>::pair()?;
        signal_hook::pipe::register(signal_hook::SIGINT, a)?;

        let (c, mut d) = Async::<UnixStream>::pair()?;
        signal_hook::pipe::register(signal_hook::SIGTERM, c)?;

        async_std::task::spawn({
            let tx = tx.clone();
            let stop_token = self.stop_token.clone();
            async move {
                // Receive a byte that indicates the SIGINT signal occurred.
                let mut read = [0];
                let read_exact = b.read_exact(&mut read).timeout_at(stop_token);

                match read_exact.await {
                    Ok(result) => {
                        //@todo obviously don't do this.
                        result.unwrap();
                        //@todo remove print here. init_env_logger in testing btw.
                        info!("Received SIGINT. Initiating shutdown...");
                        tx.lock().await.send(1).await.unwrap();
                    }
                    Err(_) => {
                        info!("Closing SIGINT watch task");
                    }
                };
            }
        });

        async_std::task::spawn({
            let stop_token = self.stop_token.clone();
            async move {
                // Receive a byte that indicates the SIGTERM signal occurred.
                let mut read = [0];
                let read_exact = d.read_exact(&mut read).timeout_at(stop_token);

                match read_exact.await {
                    Ok(result) => {
                        result.unwrap();

                        //@todo remove print here. init_env_logger in testing btw.
                        info!("Received SIGTERM. Initiating shutdown...");
                        tx.lock().await.send(1).await.unwrap();
                    }
                    Err(_) => {
                        //@todo remove print here. init_env_logger in testing btw.
                        info!("Closing SIGTERM watch task");
                    }
                }
            }
        });

        async_std::task::spawn({
            let stop_source = self.stop_source.clone();
            async move {
                //@todo maybe we want to know which signal shut us down forsure, but we can do
                //that later.
                //Wrap all this stuff in other things and maybe return a SignalEvent::Sigterm, etc
                while let Some(event) = rx.next().await {
                    info!("Shutting down due to the {} Signal received", event);
                    *stop_source.lock().await = None;
                }
            }
        });

        info!("Initialization Complete");
        Ok(())
    }

    pub fn add(&mut self, method: &str, ep: impl Endpoint<State, CState>) {
        //@todo review this code.
        let router = Arc::get_mut(&mut self.router)
            .expect("Registering routes is not possible after the Server has started");
        router.add(method, ep);
    }

    //@todo will probably change this "Endpoint" here to an upstream endpoint.
    pub fn add_upstream(&mut self, method: &str, ep: impl Endpoint<State, CState>) {
        //@todo review this code.
        let router = Arc::get_mut(&mut self.upstream_router)
            .expect("Registering routes is not possible after the Server has started");
        router.add(method, ep);
    }

    pub fn global(&mut self, global_name: &str, ep: impl Global<State, CState>) {
        let state = self.state.clone();
        let connection_list = self.connection_list.clone();
        let stop_token = self.stop_token.clone();

        let handle = async_std::task::Builder::new()
            .name(global_name.to_string())
            .spawn(async move {
                let call = ep.call(state, connection_list).timeout_at(stop_token);
                match call.await {
                    Ok(()) => {}
                    Err(e) => {
                        //@todo
                        //2. We should probably have a config setting that asks if we want to nuke everything if a
                        //   global falls. I don't know if we should automatically nuke everything, but it should
                        //   be a setting that is available. Maybe it's available on a per-global basis, but I
                        //   think it's something we should absolutely know about.
                        warn!(
                            "Global thread id: {} name: {} was unexpected closed by the error: {}",
                            async_std::task::current().id(),
                            async_std::task::current().name().unwrap_or(""),
                            e
                        );
                    }
                };
            })
            //@todo switch this to expect
            .unwrap();

        self.global_thread_list.push(handle);
    }

    pub async fn start(&mut self) -> Result<()> {
        //Initalize the recorder
        init_metrics_recorder();

        self.init().await?;

        let listen_url = format!("{}:{}", &self.host, self.port);

        let listener = TcpListener::bind(&listen_url).await?;
        let incoming = listener.incoming();

        info!("Listening on {}", &listen_url);

        let mut thread_list = Vec::new();
        let mut incoming_with_stop = incoming.timeout_at(self.stop_token.clone());

        while let Some(Ok(stream)) = incoming_with_stop.next().await {
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
            let upstream_router = self.upstream_router.clone();
            let state = self.state.clone();
            let connection_state = CState::default();

            let var_diff_config = self.var_diff_config.clone();
            let upstream_config = self.upstream_config.clone();
            let global_vars = GlobalVars::new(self.id);

            //@todo should we pass the stop token in this?
            let handle = task::spawn(async move {
                //First things first, since we get a ridiculous amount of no-data stream connections, we are
                //going to peak 2 bytes off of this and if we get those 2 bytes we continue, otherwise we are
                //burning it down with 0 logs baby.
                //@todo for safety reasons we may want to make this like 5 bytes, but thats a rare
                //scenario so I'm not too pressed for it imo.
                let mut check_bytes = [0u8; 4];
                match stream.peek(&mut check_bytes).await {
                    Err(e) => {
                        log::error!(
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
                    upstream_router,
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
            thread_list.push(handle);
        }

        let start = Instant::now();

        //@todo might be nice to have self.shutdown here?

        //Before we return Ok here, we need to finish cleaning up the rest.
        //So what I'm thinking we do is iterate through miner list and shutdown everything.
        self.connection_list.shutdown().await?;

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
        info!("Awaiting for all current connections to complete");
        for thread in thread_list {
            thread.await;
        }

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
