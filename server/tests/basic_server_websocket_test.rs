use async_std::sync::Arc;
#[cfg(feature = "websockets")]
use async_tungstenite::{async_std::connect_async, tungstenite::Message};
use stratum_server::{Connection, StratumRequest};

// pub use common::init;

pub mod common;

//@todo use ONCE here from std::sync:Once -> See the test I linked in Proq.
//@todo use future.race btw, this will call whichever function first.

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

#[derive(Clone)]
pub struct ConnectionState {}

#[cfg(feature = "websockets")]
#[async_std::test]
async fn basic_websocket_server_test() {
    let auth = AuthProvider {};
    let state = State { auth };
    let connection_state = ConnectionState {};
    let port = common::find_port().await;
    let mut server = StratumServer::builder(state, connection_state)
        .with_host("0.0.0.0")
        .with_port(port)
        .build();

    server.add("register", handle_register);

    let server = async_std::task::spawn(async move {
        server.start().await;
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

pub async fn handle_register(
    req: StratumRequest<State>,
    _connection: Arc<Connection<ConnectionState>>,
) -> Result<bool, std::io::Error> {
    let state = req.state();

    let login = state.auth.login().await;

    Ok(login)
}
