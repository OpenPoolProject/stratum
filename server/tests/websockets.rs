#[cfg(not(feature = "tcp"))]
#[async_std::test]
async fn basic_websocket_server_test() {
    use async_std::prelude::FutureExt;
    use async_tungstenite::{async_std::connect_async, tungstenite::Message};
    use futures::SinkExt;
    use std::time::Duration;
    use stratum_server::StratumServer;

    use std::sync::Arc;
    use stratum_server::{Connection, StratumRequest};

    // pub use common::init;

    pub mod common;

    //@todo use ONCE here from std::sync:Once -> See the test I linked in Proq.
    //@todo use future.race btw, this will call whichever function first.
    pub async fn handle_register(
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

    server.add("register", handle_register);

    let server = async_std::task::spawn(async move {
        server.start().await.unwrap();
    });

    let client = async_std::task::spawn(async move {
        async_std::task::sleep(Duration::from_millis(200)).await;
        //@todo we need to test wss as well.
        let (mut stream, _) = connect_async(format!("ws://0.0.0.0:{}", port))
            .await
            .unwrap();

        let msg = "{\"message\":\"register\"}";

        stream.send(Message::text(msg)).await.unwrap();
    });

    server.race(client).await;

    //@todo test getting a response message and use assert_eq.
}
