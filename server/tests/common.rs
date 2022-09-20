use async_std::{net::TcpStream, sync::Arc, task::JoinHandle};
use portpicker::pick_unused_port;
use signal_hook::{
    consts::{SIGINT, SIGTERM},
    low_level::raise,
};
use std::{sync::Once, time::Duration};
use stratum_server::{Connection, ConnectionList, StratumRequest, StratumServer};
use tracing::subscriber::set_global_default;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

pub fn init_telemetry() {
    let fmt_layer = fmt::layer();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let subscriber = Registry::default().with(filter_layer).with(fmt_layer);

    set_global_default(subscriber).expect("Failed to set subscriber");
}

pub async fn find_port() -> u16 {
    pick_unused_port().expect("No ports free")
}

static LOGGER: Once = Once::new();

pub fn init() {
    LOGGER.call_once(|| {
        init_telemetry();
    });
}

pub fn call_sigint() {
    tracing::info!("Raising SIGINT signal");
    raise(SIGINT).unwrap();
}

pub fn call_sigterm() {
    tracing::info!("Raising SIGTERM signal");
    raise(SIGTERM).unwrap();
}

#[derive(Clone)]
pub struct AuthProvider {}

impl AuthProvider {
    pub async fn login(&self) -> bool {
        true
    }
}

#[derive(Clone)]
pub struct State {
    auth: AuthProvider,
}

#[derive(Clone, Default)]
pub struct ConnectionState {}

//@todo test returning a message, so that we can assert eq in the main test.
pub async fn handle_auth(
    req: StratumRequest<State>,
    _connection: Arc<Connection<ConnectionState>>,
) -> Result<bool, std::io::Error> {
    let state = req.state();

    let login = state.auth.login().await;

    Ok(login)
}

pub async fn poll_global(_state: State, _connection_list: Arc<ConnectionList<ConnectionState>>) {
    loop {
        //Infite loop
        async_std::task::sleep(Duration::from_secs(10)).await;
    }
}

pub async fn server_with_auth(port: u16) -> StratumServer<State, ConnectionState> {
    let auth = AuthProvider {};
    let state = State { auth };
    // let port = find_port().await;
    let mut server = StratumServer::builder(state, 1)
        .with_host("0.0.0.0")
        .with_port(port)
        .build();

    server.add("auth", handle_auth);

    server
}

pub async fn server_with_global(port: u16) -> StratumServer<State, ConnectionState> {
    let auth = AuthProvider {};
    let state = State { auth };
    // let port = find_port().await;
    let mut server = StratumServer::builder(state, 1)
        .with_host("0.0.0.0")
        .with_port(port)
        .build();

    server.add("auth", handle_auth);
    server.global("Poll Global", poll_global);

    server
}

//@note these connections do not send any messages.
#[cfg(not(feature = "websocket"))]
pub fn generate_connections(num: usize, url: &str, sleep_duration: u64) -> Vec<JoinHandle<usize>> {
    let mut connections = Vec::new();

    for i in 0..num {
        let client = async_std::task::spawn({
            let url = url.to_string();
            async move {
                //Setup Costs
                async_std::task::sleep(Duration::from_millis(200)).await;

                let _stream = TcpStream::connect(&url).await.unwrap();

                async_std::task::sleep(Duration::from_secs(sleep_duration)).await;

                i
            }
        });

        connections.push(client);
    }

    connections
}

//@todo This needs to work.
#[cfg(feature = "websocket")]
pub fn generate_connections(num: usize, url: &str, sleep_duration: u64) -> Vec<JoinHandle<usize>> {
    let mut connections = Vec::new();

    for i in 0..num {
        let client = async_std::task::spawn({
            let url = url.to_string();
            async move {
                //Setup Costs
                async_std::task::sleep(Duration::from_millis(200)).await;

                let mut stream = TcpStream::connect(&url).await.unwrap();
                async_tungstenite::async_std::task::sleep(Duration::from_secs(sleep_duration))
                    .await;
                // async_tungstenite::

                i
            }
        });

        connections.push(client);
    }

    connections
}
