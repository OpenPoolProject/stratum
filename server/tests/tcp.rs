pub mod common;

#[async_std::test]
async fn test_basic_server() {
    #[cfg(not(target_env = "msvc"))]
    use jemallocator::Jemalloc;

    #[cfg(not(target_env = "msvc"))]
    #[global_allocator]
    static GLOBAL: Jemalloc = Jemalloc;

    use async_std::{net::TcpStream, prelude::FutureExt, sync::Arc};
    use futures::io::AsyncWriteExt;
    // use jemallocator::Jemalloc;
    use std::time::Duration;
    use stratum_server::{Connection, StratumRequest, StratumServer};

    // pub use common::init;

    //@todo note: We might be suffering from this: https://github.com/rust-lang/cargo/issues/7916 AND https://github.com/rust-lang/cargo/issues/1796
    //Due to the fact that we are including async-std twice, only with a different feature in the
    //development mode. This is not a huge deal, but it's probably important to monitor the status on
    //the above issues.

    //@todo use ONCE here from std::sync:Once -> See the test I linked in Proq.
    //@todo use future.race btw, this will call whichever function first.
    //
    //@todo move handle_auth and the server setup to common.
    //@todo also should probs just select! instead of race?
    //
    //@todo test for the 10 minute delay killing. We can do this by making sure that the 10 minute
    //stale timing is configurable.
    //@todo test 2 connections, dropping one, and then testing that there is 1 left (to ensure we don't
    //drop everything).
    //@todo test 1 connection and dropping.

    // #[cfg(not(target_env = "msvc"))]
    // #[global_allocator]
    // static GLOBAL: Jemalloc = Jemalloc;

    pub async fn handle_auth(
        req: StratumRequest<State>,
        _connection: Arc<Connection<ConnectionState>>,
    ) -> Result<bool, std::io::Error> {
        let state = req.state();

        let login = state.auth.login().await;

        Ok(login)
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
    common::init();

    let auth = AuthProvider {};
    let state = State { auth };
    let port = common::find_port().await;
    let mut server = StratumServer::builder(state, 1)
        .with_host("0.0.0.0")
        .with_port(port)
        .build();

    server.add("auth", handle_auth);

    let server = async_std::task::spawn(async move {
        server.start().await.unwrap();
        0
    });

    let client = async_std::task::spawn(async move {
        async_std::task::sleep(Duration::from_millis(200)).await;
        let mut stream = TcpStream::connect(format!("0.0.0.0:{port}")).await.unwrap();
        let msg = "{\"method\":\"auth\"}";

        stream.write_all(msg.as_bytes()).await.unwrap();
        stream.write_all(b"\n").await.unwrap();

        1
    });

    let result = server.race(client).await;

    assert_eq!(result, 1);
}

//#[async_std::test]
//async fn test_single_connection_shutdown() {
//    common::init();
//    let auth = AuthProvider {};
//    let state = State { auth };
//    let connection_state = ConnectionState {};
//    let port = common::find_port().await;
//    let mut server = StratumServer::builder(state, connection_state)
//        .with_host("0.0.0.0")
//        .with_port(port)
//        .build();

//    server.add("auth", handle_auth);

//    let client = async_std::task::spawn(async move {
//        //Time to let the server setup.
//        async_std::task::sleep(Duration::from_secs(4)).await;

//        let mut stream = TcpStream::connect(format!("0.0.0.0:{}", port))
//            .await
//            .unwrap();
//        let msg = "{\"method\":\"auth\"}";

//        stream.write_all(msg.as_bytes()).await;
//        stream.write_all(b"\n").await;

//        1_u32
//    });

//    let miner_list = server.get_miner_list();
//    assert_eq!(miner_list.len().await, 0);

//    //@todo sidenote, I can get miner list before I spawn this task.
//    let server = async_std::task::spawn(async move {
//        server.start().await;
//    });

//    //Give the server time to register the hooks.
//    async_std::task::sleep(Duration::from_secs(2)).await;
//    assert_eq!(miner_list.len().await, 1);

//    //@todo wrap this in common helper functions?
//    // println!("Raising SIGINT signal");

//    let result = server.await;
//    // let result = server.race(client).await;

//    // assert!(result.is_ok());
//}

//@todo test returning a message, so that we can assert eq in the main test.
