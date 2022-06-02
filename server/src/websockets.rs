pub use crate::ConnectionList;
use crate::{
    config::{UpstreamConfig, VarDiffConfig},
    connection::{Connection, SendInformation},
    id_manager::IDManager,
    router::Router,
    types::{ExMessageGeneric, GlobalVars, MessageValue},
    BanManager, Error, Result, EX_MAGIC_NUMBER,
};
use async_std::{net::TcpStream, prelude::FutureExt, sync::Arc};
use async_tungstenite::{tungstenite::protocol::Message, WebSocketStream};
use extended_primitives::Buffer;
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
    io::{AsyncBufReadExt, AsyncReadExt, BufReader, ReadHalf, WriteHalf},
    stream::{SplitSink, SplitStream},
    AsyncWriteExt, SinkExt, StreamExt,
};
use log::{trace, warn};
use serde_json::{Map, Value};
use std::net::SocketAddr;
use stop_token::future::FutureExt as stopFutureExt;

//@todo might make sene to wrap a lot of these into one param called "ConnectionConfig" and then
//just pass that along, but we'll see.
#[allow(clippy::too_many_arguments)]
pub async fn handle_connection<
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
>(
    id_manager: Arc<IDManager>,
    ban_manager: Arc<BanManager>,
    mut addr: SocketAddr,
    connection_list: Arc<ConnectionList<CState>>,
    router: Arc<Router<State, CState>>,
    upstream_router: Arc<Router<State, CState>>,
    upstream_config: UpstreamConfig,
    state: State,
    stream: TcpStream,
    var_diff_config: VarDiffConfig,
    initial_difficulty: u64,
    connection_state: CState,
    _proxy: bool,
    _expected_port: u16,
    global_vars: GlobalVars,
) -> Result<()> {
    //@todo through this error don't call expect
    let stream = async_tungstenite::accept_async(stream)
        .await
        .expect("Error during the websocket handshake occurred");

    let (wh, mut rh) = stream.split();

    let mut buffer_stream = rh;
    // let mut buffer_stream = BufReader::new(rh);

    if ban_manager.check_banned(&addr).await {
        warn!(
            "Banned connection attempting to connect: {}. Connection closed",
            addr
        );

        return Ok(());
    }
    let (tx, rx) = unbounded();
    let (utx, urx) = unbounded();
    let (urtx, urrx) = unbounded();

    //@todo we should be printing the number of sessions issued out of the total supported.
    //Currently have 24 sessions connected out of 15,000 total. <1% capacity.
    let connection_id = match id_manager.allocate_session_id().await {
        Some(id) => id,
        None => {
            warn!("Sessions full");
            return Ok(());
        }
    };

    let connection = Arc::new(Connection::new(
        connection_id,
        tx,
        utx,
        urrx,
        initial_difficulty,
        var_diff_config,
        connection_state,
    ));

    let stop_token = connection.get_stop_token();

    let id = connection.id();

    async_std::task::spawn(async move {
        match send_loop(rx, wh).await {
            //@todo we should make this conditional on the connection actually being legit, or we
            //can also check before we make a connection so we dodge all these nastiness
            Ok(_) => trace!("Send Loop is closing for connection: {}", id),
            Err(e) => warn!("Send loop is closed for connection: {}, Reason: {}", id, e),
        }
    });

    //@todo handle this undwrap?
    connection_list
        .add_miner(addr, connection.clone())
        .await
        .unwrap();

    loop {
        if connection.is_disconnected().await {
            trace!(
                "Connection: {} disconnected. Breaking out of next_message loop",
                connection.id()
            );
            break;
        }

        let timeout = connection.timeout().await;

        let next_message = next_message(&mut buffer_stream)
            .timeout(timeout)
            .timeout_at(stop_token.clone())
            .await;

        match next_message {
            //@todo this would most likely be stop_token
            Err(e) => log::error!(
                "Connection: {} error in 'next_message' (stop_token) Error: {}",
                connection.id(),
                e
            ),
            Ok(msg) => {
                //@todo this would most likely be timeout function
                match msg {
                    Err(e) => {
                        log::error!(
                            "Connection: {} error in 'next_message' (timeout fn) Error: {}",
                            connection.id(),
                            e
                        );
                        break;
                    }
                    Ok(msg) => match msg {
                        Err(e) => {
                            log::error!(
                                "Connection: {} error in 'next_message' (decoding/reading) Error: {}",
                                connection.id(), e
                            );
                            break;
                        }
                        Ok((method, values)) => {
                            router
                                .call(
                                    &method,
                                    values,
                                    state.clone(),
                                    connection.clone(),
                                    global_vars.clone(),
                                )
                                .await;
                        }
                    },
                }
            }
        }
    }

    //@todo I think we should try to move these log statements into the Connection, since when they
    //are just out here, we print them even when it's a bogus connection.
    //@todo on that note, let's go through this workflow as if we are a complete hack and see if we
    //can figure out if there are any bad spots.
    //Not necessarily a hack, but say like a random request from a random website.
    trace!("Closing stream from: {}", connection.id());

    id_manager.remove_session_id(connection_id).await;
    connection_list.remove_miner(addr).await;

    if connection.needs_ban().await {
        ban_manager.add_ban(&addr).await;
    }

    connection.shutdown().await;

    Ok(())
}

//@todo a couple of things... We need to prevent attacks against us. This is a niche thing, but we
//need to prevent buffer overflows here.
//Right now I'm going to enable some sketchy shit for devfees on nimiq, but I belive that we should
//have a limit where we check what works and what doesn't in terms of how much non-conforming data
//we allow before we start closign the sockets.
pub async fn next_message(
    stream: &mut SplitStream<WebSocketStream<TcpStream>>,
) -> Result<(String, MessageValue)> {
    let msg = match stream.next().await {
        //@todo this can broken pipe here, we want to just return an error I think so we can drop
        //this connection.
        Some(msg) => msg.unwrap(),
        None => {
            return Err(Error::StreamClosed(format!("Websocket closed")));
        }
    };

    let raw = msg.into_text().unwrap();

    //I don't actually think this has to loop here.

    trace!("Received Message: {}", &raw);

    let msg: Map<String, Value> = match serde_json::from_str(&raw) {
        Ok(msg) => msg,
        Err(_) => return Err(Error::MethodDoesntExist),
    };

    let method = if msg.contains_key("method") {
        match msg.get("method") {
            Some(method) => method.as_str(),
            //@todo need better stratum erroring here.
            None => return Err(Error::MethodDoesntExist),
        }
    } else if msg.contains_key("messsage") {
        match msg.get("message") {
            Some(method) => method.as_str(),
            None => return Err(Error::MethodDoesntExist),
        }
    } else if msg.contains_key("result") {
        Some("result")
    } else {
        // return Err(Error::MethodDoesntExist);
        Some("")
    };

    if let Some(method_string) = method {
        //Mark the sender as active as we received a message.
        //We only mark them as active if the message/method was valid
        // self.stats.lock().await.last_active = Utc::now().naive_utc();
        // @todo maybe expose a function on the connection for this btw.

        return Ok((method_string.to_owned(), MessageValue::StratumV1(msg)));
    } else {
        //@todo improper format
        return Err(Error::MethodDoesntExist);
    }
}

//@todo this should return a result and we should catch on these others.
pub async fn send_loop(
    mut rx: UnboundedReceiver<SendInformation>,
    mut rh: SplitSink<WebSocketStream<TcpStream>, Message>,
) -> Result<()> {
    while let Some(msg) = rx.next().await {
        match msg {
            SendInformation::Json(json) => {
                //@todo
                rh.send(Message::Text((json.as_str().to_owned()))).await?;
            }
            SendInformation::Text(text) => rh.send(Message::Text((text))).await?,
            SendInformation::Raw(buffer) => rh.send(Message::Binary((buffer.to_vec()))).await?,
        }
        // wh.send(Message::Text(msg)).await?;
    }

    //Close message
    rh.send(Message::Close(None)).await?;
    Ok(())
}

pub async fn upstream_send_loop(
    mut rx: UnboundedReceiver<String>,
    mut rh: WriteHalf<TcpStream>,
) -> Result<()> {
    while let Some(msg) = rx.next().await {
        rh.write_all(msg.as_bytes()).await?;
        rh.write_all(b"\n").await?;
    }

    Ok(())
}
