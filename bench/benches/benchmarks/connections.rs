use async_std::{
    net::TcpStream,
    sync::Arc,
    task::{block_on, JoinHandle},
};
use criterion::{criterion_group, BenchmarkId, Criterion};
use std::time::Duration;
use stratum_server::{Connection, StratumRequest, StratumServer};

pub fn generate_connections(num: usize, url: &str, sleep_duration: u64) -> Vec<JoinHandle<usize>> {
    let mut connections = Vec::new();

    for i in 0..num {
        let client = async_std::task::spawn({
            let url = url.to_string();
            async move {
                //Setup Costs
                async_std::task::sleep(Duration::from_millis(200)).await;

                let _stream = TcpStream::connect(&url).await;

                async_std::task::sleep(Duration::from_secs(sleep_duration)).await;

                i
            }
        });

        connections.push(client);
    }

    connections
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

fn num_connections(c: &mut Criterion) {
    let _server = block_on(async { server_with_auth(8889).await });

    let num: usize = 1;

    c.bench_with_input(BenchmarkId::new("num_connections", num), &num, |b, &s| {
        b.iter(|| generate_connections(s, "0.0.0.0:8889", 100));
    });

    // let mut group = c.benchmark_group("\"*group/\"");
    // group.bench_function("\"*benchmark/\" '", |b| b.iter(|| 1 + 1));
    // group.finish();
}

criterion_group! {
    name = benches;
    // This can be any expression that returns a `Criterion` object.
    config = Criterion::default().measurement_time(Duration::from_millis(10));
    targets = num_connections
}

// criterion_group!(benches, num_connections);
// criterion_main!(benches);
