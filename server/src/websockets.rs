use crate::connection::Connection;
use crate::router::Router;
use crate::server::{UpstreamConfig, VarDiffConfig};
use crate::BanManager;
pub use crate::MinerList;
use crate::{Error, Result};
use async_std::net::TcpStream;
use async_std::sync::Arc;
use async_tungstenite::tungstenite::protocol::Message;
use async_tungstenite::WebSocketStream;
use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::stream::{SplitSink, SplitStream};
use futures::SinkExt;
use futures::StreamExt;
use log::{debug, info, warn};
use serde_json::{Map, Value};
use std::net::SocketAddr;

#[allow(clippy::too_many_arguments)]
pub async fn handle_connection<
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
>(
    ban_manager: Arc<BanManager>,
    addr: SocketAddr,
    connection_list: Arc<MinerList>,
    router: Arc<Router<State, CState>>,
    upstream_router: Arc<Router<State, CState>>,
    upstream_config: UpstreamConfig,
    state: State,
    // stream: WebSocketStream<TcpStream>,
    stream: TcpStream,
    var_diff_config: VarDiffConfig,
    initial_difficulty: f64,
    connection_state: CState,
    //@todo we don't use these in websockets, figure out if we need to or a better way to handle
    //this? Otherwise just double check this works.
    _proxy: bool,
    _expected_port: u16,
) {
    let stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (wh, mut rh) = stream.split();
    //
    // let mut rh = BufReader::new(rh);
    // @todo upstream is not configured correctly in this file I believe, so that is something we
    // need to fix.

    // let mut rh = if !cfg!(feature = "websockets") {
    //     BufReader::new(rh)
    // } else {
    //     rh
    // };

    if !ban_manager.check_banned(&addr).await {
        //@todo put this into the connection.
        let (tx, rx) = unbounded();
        let (utx, urx) = unbounded();
        let (mut urtx, urrx) = unbounded();

        let connection = Arc::new(Connection::new(
            addr,
            tx,
            utx,
            urrx,
            // buffer_stream,
            // wh,
            // @todo these should all come in some config.
            initial_difficulty,
            var_diff_config,
            connection_state,
        ));

        async_std::task::spawn(async move {
            match send_loop(rx, wh).await {
                //@todo not sure if we even want a info here, we need an ID tho.
                Ok(_) => info!("Send Loop is closing for connection"),
                Err(e) => warn!("Send loop is closed for connection: {}, Reason: {}", 1, e),
            }
        });

        connection_list
            .add_miner(addr, connection.clone())
            .await
            .unwrap();

        info!("Accepting stream from: {}", addr);

        loop {
            if connection.is_disconnected().await {
                break;
            }

            //Maybe have a wrap here or something and on Error instead of unwrap we
            //break.
            // let (method, values) = match connection.next_message().await {
            //     Ok((method, values)) => (method, values),
            //     Err(_) => {
            //         break;
            //     }
            // };
            let (method, values) = match next_message(&mut rh).await {
                Ok((method, values)) => (method, values),
                Err(_) => {
                    break;
                }
            };

            router
                .call(&method, values, state.clone(), connection.clone())
                .await;
        }

        //@todo we can kill state in connection now
        //Unused for now, but may be useful for logging or bans
        // let _result = connection.start(router.clone()).await;

        info!("Closing stream from: {}", addr);

        //First thing is let's kill the send loop.

        // wh.send(Message::Close(None)).await;

        //Ideally don't unwrap here.
        // let mut original = rh.reunite(wh).unwrap();

        // original.close(None).await;

        connection_list.remove_miner(addr).await.unwrap();

        if connection.needs_ban().await {
            ban_manager.add_ban(&addr).await;
        }

    // drop(connection);

    // send_loop.await;
    } else {
        warn!(
            "Banned connection attempting to connect: {}. Connected closed",
            addr
        );
    }
}

//@todo a couple of things... We need to prevent attacks against us. This is a niche thing, but we
//need to prevent buffer overflows here.
//Right now I'm going to enable some sketchy shit for devfees on nimiq, but I belive that we should
//have a limit where we check what works and what doesn't in terms of how much non-conforming data
//we allow before we start closign the sockets.
pub async fn next_message(
    rh: &mut SplitStream<WebSocketStream<TcpStream>>,
) -> Result<(String, serde_json::map::Map<String, serde_json::Value>)> {
    //@todo do a match here on the type of message as well. Close on close message.
    let msg = match rh.next().await {
        //@todo this can broken pipe here, we want to just return an error I think so we can drop
        //this connection.
        Some(msg) => msg.unwrap(),
        None => {
            return Err(Error::StreamClosed);
        }
    };

    let raw = msg.into_text().unwrap();

    debug!("Received Message: {}", &raw);

    //let raw = match msg.text() {
    //    Some(text) => text.unwrap(),
    //    None => {
    //        //@todo double check this.
    //        return Err(Error::StreamClosed);
    //    }
    //};

    let msg: Map<String, Value> = serde_json::from_str(&raw)?;

    let method = if msg.contains_key("method") {
        match msg.get("method") {
            Some(method) => method.as_str(),
            //@todo need better stratum erroring here.
            None => return Err(Error::MethodDoesntExist),
        }
    } else if msg.contains_key("message") {
        match msg.get("message") {
            Some(method) => method.as_str(),
            //@todo need better stratum erroring here.
            None => return Err(Error::MethodDoesntExist),
        }
    } else {
        // return Err(Error::MethodDoesntExist);
        Some("")
    };

    if let Some(method_string) = method {
        //Mark the sender as active as we received a message.
        //We only mark them as active if the message/method was valid
        // self.stats.lock().await.last_active = Utc::now().naive_utc();

        Ok((method_string.to_owned(), msg))
    } else {
        //@todo improper format
        Err(Error::MethodDoesntExist)
    }
}

//@todo this should return a result and we should catch on these others.
pub async fn send_loop(
    mut rx: UnboundedReceiver<String>,
    mut wh: SplitSink<WebSocketStream<TcpStream>, Message>,
) -> Result<()> {
    while let Some(msg) = rx.next().await {
        wh.send(Message::Text(msg)).await?;
    }

    //Close message
    wh.send(Message::Close(None)).await?;
    Ok(())
}
