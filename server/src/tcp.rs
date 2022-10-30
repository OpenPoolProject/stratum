#[cfg(feature = "upstream")]
use {crate::config::UpstreamConfig, crate::upstream::upstream_message_handler};

use crate::{
    config::VarDiffConfig,
    connection::{Connection, SendInformation},
    id_manager::IDManager,
    parsing::{next_message, proxy_protocol},
    router::Router,
    types::GlobalVars,
    BanManager, ConnectionList, Result,
};
use futures::{
    channel::mpsc::{unbounded, UnboundedReceiver},
    StreamExt,
};
use std::{net::SocketAddr, sync::Arc};
use tokio::{
    io::{AsyncWriteExt, BufReader},
    net::{tcp::OwnedWriteHalf, TcpStream},
};
use tracing::{error, trace, warn};

//@note / @todo I think this is the play in that for each "protocol" we implement a Handler (does
//message parsing and State management) and a "Connection" (different than our courrent one) which
//wraps whatever medium we use to connect e.g. v1 - base tcp, v2 - noise, autonomy - brontide, e.g.
//Then we can later make it generic so that we can re-implement these things for stuff like Nimiq
//and websockets/etc.
pub(crate) struct Handler<State, CState>
where
    CState: Send + Sync + Clone + 'static,
{
    pub(crate) id_manager: Arc<IDManager>,
    pub(crate) ban_manager: Arc<BanManager>,
    pub(crate) connection_list: Arc<ConnectionList<CState>>,
    pub(crate) router: Arc<Router<State, CState>>,
    pub(crate) state: State,
    //@todo might make sense to rewrap this in a new struct (not Connection) Or rename Connection
    //to Stratum Connection.
    pub(crate) stream: TcpStream,
    //@todo might want to have this link to "ConfigManager" so that A. we can get updates on config
    //reloads, and B. makes it easier to transfer some of these threads.
    pub(crate) var_diff_config: VarDiffConfig,
    pub(crate) initial_difficulty: u64,
    pub(crate) connection_state: CState,
    pub(crate) proxy: bool,
    pub(crate) expected_port: u16,
    pub(crate) global_vars: GlobalVars,
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    Handler<State, CState>
{
    pub(crate) async fn run(&self) -> Result<()> {
        Ok(())
    }
}

//@todo I big think that I think we need to focus on today is catching attacks like open sockets
//doing nothing, socketrs trying to flood, etc.
//Let's make sure we have an entire folder of tests for "attacks" and make sure that we cover them
//thoroughly.

//@todo might make sene to wrap a lot of these into one param called "ConnectionConfig" and then
//just pass that along, but we'll see.
//@todo review
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
    let (rh, wh) = stream.into_split();

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

    //@todo wrap all upstream shit in init_upstream();

    let (tx, rx) = unbounded();
    // @todo we should have a function that returns this.
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

    let cancel_token = connection.get_cancel_token();

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

    tokio::task::spawn(async move {
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
        //@todo we could possibly do something like Cancellation token and see miniRedis from tokyo
        //where this would be a future, so we can put it into select!
        if connection.is_disconnected().await {
            trace!(
                "Connection: {} disconnected. Breaking out of next_message loop",
                connection.id()
            );
            break;
        }

        let timeout = connection.timeout().await;

        tokio::select! {
        //@todo try this suggestion later.
        //If this returns first, it's either a Timeout, or successful message read.
        //We should also try the "else" method here so we would match Ok(msg) = and then cancel
        //match, and then the else would be a timeout message which would match Err(msg)
        res = tokio::time::timeout(timeout, next_message(&mut buffer_stream)) => {

                    //Next_message Success
                    if let Ok(result) = res {
                    match result {
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
                    },
                    Err(e) => {
                        error!(
                            "Connection: {} error in 'next_message' (decoding/reading) Error: {}",
                            connection.id(), e
                        );
                        break;
                }

                    }

                    } else {
            error!(connection_id=connection.id().to_string(), timeout=timeout.as_secs(), "next_message timed out.");

                }
            }
        _ = cancel_token.cancelled() => {
            error!(connection_id=connection.id().to_string(), "Message parsing canceled. Received Shutdown");
            break;
        }

        }
    }

    // let next_message = tokio::time::timeout(timeout, next_message(&mut buffer_stream))
    //     .await;
    //
    // match next_message {
    //     //@todo this would most likely be stop_token
    //     Err(e) => tracing::error!(
    //         "Connection: {} error in 'next_message' (stop_token) Error: {}",
    //         connection.id(),
    //         e
    //     ),
    //     Ok(msg) => {
    //         //@todo this would most likely be timeout function
    //         match msg {
    //             Err(e) => {
    //                 tracing::error!(
    //                     "Connection: {} error in 'next_message' (timeout fn) Error: {}",
    //                     connection.id(),
    //                     e
    //                 );
    //                 break;
    //             }
    //             Ok(msg) => match msg {
    //                 Err(e) => {
    //                     tracing::error!(
    //                         "Connection: {} error in 'next_message' (decoding/reading) Error: {}",
    //                         connection.id(), e
    //                     );
    //                     break;
    //                 }
    //                 Ok((method, values)) => {
    //                     router
    //                         .call(
    //                             &method,
    //                             values,
    //                             state.clone(),
    //                             connection.clone(),
    //                             global_vars.clone(),
    //                         )
    //                         .await;
    //                 }
    //             },
    //         }
    //     }
    // }

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
    mut rh: OwnedWriteHalf,
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
    }
    Ok(())
}
