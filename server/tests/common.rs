use signal_hook::{
    consts::{SIGINT, SIGTERM},
    low_level::raise,
};
use std::{
    net::SocketAddr,
    sync::{Arc, Once},
    time::Duration,
};
use stratum_server::{Connection, ConnectionList, Result, StratumRequest, StratumServer};
use tokio::{net::TcpStream, task::JoinHandle, time::sleep};
use tracing::subscriber::set_global_default;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

pub const STARTUP_TIME: Duration = Duration::from_secs(2);
//@todo reduce this.
pub const CONNECTION_DELAY: Duration = Duration::from_secs(1);

pub fn init_telemetry() {
    let fmt_layer = fmt::layer();
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    let subscriber = Registry::default().with(filter_layer).with(fmt_layer);

    set_global_default(subscriber).expect("Failed to set subscriber");
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
) -> Result<bool> {
    let state = req.state();

    let login = state.auth.login().await;

    Ok(login)
}

pub async fn poll_global(_state: State, _connection_list: Arc<ConnectionList<ConnectionState>>) {
    loop {
        //Infite loop
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
}

// pub async fn server_with_auth(port: u16) -> StratumServer<State, ConnectionState> {
//     let auth = AuthProvider {};
//     let state = State { auth };
//     let mut server = StratumServer::builder(state, 1)
//         .with_host("0.0.0.0")
//         .with_port(port)
//         .build()
//         .await?;
//
//     server.add("auth", handle_auth);
//
//     server
// }

// pub async fn server_with_global(port: u16) -> StratumServer<State, ConnectionState> {
//     let auth = AuthProvider {};
//     let state = State { auth };
//     // let port = find_port().await;
//     let mut server = StratumServer::builder(state, 1)
//         .with_host("0.0.0.0")
//         .with_port(port)
//         .build();
//
//     server.add("auth", handle_auth);
//     server.global("Poll Global", poll_global);
//
//     server
// }

//@todo this JoinHandle should return a Result, and we should check to make sure its a shutdonw
//error in the signal tests.
pub async fn spawn_full_server() -> Result<(SocketAddr, JoinHandle<Result<()>>)> {
    let auth = AuthProvider {};
    let state = State { auth };
    // let port = find_port().await;
    let builder = StratumServer::builder(state, 1)
        .with_host("0.0.0.0")
        .with_port(0);

    #[cfg(feature = "api")]
    let builder = builder.with_api_host("0.0.0.0").with_api_port(0);

    let mut server = builder.build().await?;

    let address = server.get_address();

    server.add("auth", handle_auth);
    server.global("Poll Global", poll_global);

    let handle = tokio::spawn(async move { server.start().await });

    sleep(STARTUP_TIME).await;

    Ok((address, handle))
}

//@note these connections do not send any messages.
pub async fn generate_connections<A: Into<SocketAddr>>(
    num: usize,
    url: A,
    sleep_duration: u64,
) -> Vec<JoinHandle<usize>> {
    let addrs = url.into();
    let mut connections = Vec::new();

    for i in 0..num {
        let client = tokio::task::spawn({
            async move {
                let _stream = TcpStream::connect(addrs).await.unwrap();

                tokio::time::sleep(Duration::from_secs(sleep_duration)).await;

                i
            }
        });

        connections.push(client);
    }

    sleep(CONNECTION_DELAY).await;

    connections
}
