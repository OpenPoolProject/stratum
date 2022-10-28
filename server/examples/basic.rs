use async_std::{sync::Arc, task};
use std::time::Duration;
use stratum_server::{Connection, ConnectionList, StratumRequest, StratumServer};
use tracing::subscriber::set_global_default;
use tracing_subscriber::{fmt, prelude::*, EnvFilter, Registry};

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

fn main() {
    task::block_on(async {
        let fmt_layer = fmt::layer();
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("info"))
            .unwrap();

        let subscriber = Registry::default().with(filter_layer).with(fmt_layer);

        set_global_default(subscriber).expect("Failed to set subscriber");
        let auth = AuthProvider {};
        let state = State { auth };
        // let port = find_port().await;
        let mut server = StratumServer::builder(state, 1)
            .with_host("0.0.0.0")
            .with_port(0)
            .build();

        server.add("auth", handle_auth);
        server.global("Poll Global", poll_global);

        server.start().await.unwrap();
    });
}
