#[cfg(feature = "upstream")]
use {crate::config::UpstreamConfig, crate::upstream::upstream_message_handler};

pub use crate::ConnectionList;
use crate::{
    config::VarDiffConfig,
    connection::{Connection, SendInformation},
    id_manager::IDManager,
    next_message,
    router::Router,
    types::GlobalVars,
    BanManager, Error, Result,
};
use async_std::{net::TcpStream, prelude::FutureExt, sync::Arc};
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver},
    io::{AsyncBufReadExt, AsyncReadExt, BufReader, ReadHalf, WriteHalf},
    AsyncWriteExt, StreamExt,
};
use std::net::SocketAddr;
use stop_token::future::FutureExt as stopFutureExt;
use tracing::{trace, warn};

//@todo move this to parsing.
pub async fn proxy_protocol(
    buffer_stream: &mut BufReader<ReadHalf<TcpStream>>,
    expected_port: u16,
) -> Result<SocketAddr> {
    let mut buf = String::new();

    buffer_stream.read_line(&mut buf).await.unwrap();

    //Buf will be of the format "PROXY TCP4 92.118.161.17 172.20.42.228 55867 8080\r\n"
    //Trim the \r\n off
    let buf = buf.trim();
    //Might want to not be ascii whitespace and just normal here.
    // let pieces = buf.split_ascii_whitespace();

    let pieces: Vec<&str> = buf.split(' ').collect();

    let attempted_port: u16 = pieces[5].parse().unwrap();

    //Check that they were trying to connect to us.
    if attempted_port != expected_port {
        return Err(Error::StreamWrongPort);
    }

    Ok(format!("{}:{}", pieces[2], pieces[4]).parse()?)
}

//@todo might make sene to wrap a lot of these into one param called "ConnectionConfig" and then
//just pass that along, but we'll see.
//@todo review
#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
pub async fn handle_connection<
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
>(
    id_manager: Arc<IDManager>,
    ban_manager: Arc<BanManager>,
    mut addr: SocketAddr,
    connection_list: Arc<ConnectionList<CState>>,
    router: Arc<Router<State, CState>>,

    #[cfg(feature = "upstream")] upstream_router: Arc<Router<State, CState>>,
    #[cfg(feature = "upstream")] upstream_config: UpstreamConfig,
    state: State,
    stream: TcpStream,
    var_diff_config: VarDiffConfig,
    initial_difficulty: u64,
    connection_state: CState,
    proxy: bool,
    expected_port: u16,
    global_vars: GlobalVars,
) -> Result<()> {
    let (rh, wh) = stream.split();

    let mut buffer_stream = BufReader::new(rh);

    if proxy {
        addr = proxy_protocol(&mut buffer_stream, expected_port).await?;
    }

    if ban_manager.check_banned(&addr).await {
        warn!(
            "Banned connection attempting to connect: {}. Connection closed",
            addr
        );

        return Ok(());
    }

    let (tx, rx) = unbounded();
    #[cfg(feature = "upstream")]
    let (utx, urx) = unbounded();
    #[cfg(feature = "upstream")]
    let (urtx, urrx) = unbounded();

    //@todo we should be printing the number of sessions issued out of the total supported.
    //Currently have 24 sessions connected out of 15,000 total. <1% capacity.

    let connection_id = if let Some(id) = id_manager.allocate_session_id().await {
        id
    } else {
        warn!("Sessions full");
        return Ok(());
    };

    let connection = Arc::new(Connection::new(
        connection_id,
        tx,
        #[cfg(feature = "upstream")]
        utx,
        #[cfg(feature = "upstream")]
        urrx,
        initial_difficulty,
        var_diff_config,
        connection_state,
    ));

    let stop_token = connection.get_stop_token();

    #[cfg(feature = "upstream")]
    upstream_message_handler(
        upstream_config,
        upstream_router,
        urx,
        state.clone(),
        connection.clone(),
        urtx,
        global_vars.clone(),
    )
    .await?;

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
            Err(e) => tracing::error!(
                "Connection: {} error in 'next_message' (stop_token) Error: {}",
                connection.id(),
                e
            ),
            Ok(msg) => {
                //@todo this would most likely be timeout function
                match msg {
                    Err(e) => {
                        tracing::error!(
                            "Connection: {} error in 'next_message' (timeout fn) Error: {}",
                            connection.id(),
                            e
                        );
                        break;
                    }
                    Ok(msg) => match msg {
                        Err(e) => {
                            tracing::error!(
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

        //@todo maybe do triple ??? instead?
        //@todo I don't think we like the triple ??? actually because we want to break the loop and
        //not automatically complete the function so we can do shutdown proceedures.
        //Check to see if we did ? anywhere, and if so let's fix that.
        // if let Ok(Ok(Ok((method, values)))) = next_message {
        //     router
        //         .call(
        //             &method,
        //             values,
        //             state.clone(),
        //             connection.clone(),
        //             global_vars.clone(),
        //         )
        //         .await;
        // } else {
        //     break;
        // }
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

pub async fn send_loop(
    mut rx: UnboundedReceiver<SendInformation>,
    mut rh: WriteHalf<TcpStream>,
) -> Result<()> {
    while let Some(msg) = rx.next().await {
        match msg {
            SendInformation::Json(json) => {
                rh.write_all(json.as_bytes()).await?;
                rh.write_all(b"\n").await?;
            }
            SendInformation::Text(text) => {
                rh.write_all(text.as_bytes()).await?;
            }
            SendInformation::Raw(buffer) => {
                rh.write_all(&buffer).await?;
            }
        }

        // rh.write_all(msg.as_bytes()).await?;
        //@todo the reason we write this here is that JSON RPC messages are ended with a newline.
        //This probably should be built into the rpc library, but it works here for now.
        //Don't move this unless websockets ALSO require the newline, then we can move it back into
        //the Connection.send function.
        // rh.write_all(b"\n").await?;
    }

    Ok(())
}
