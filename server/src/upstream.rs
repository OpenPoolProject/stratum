pub use crate::ConnectionList;
use crate::{
    config::UpstreamConfig,
    connection::Connection,
    next_message,
    router::Router,
    types::{GlobalVars, MessageValue},
    Error, Result,
};
use async_std::{net::TcpStream, sync::Arc};
use futures::{
    channel::mpsc::{UnboundedReceiver, UnboundedSender},
    io::{AsyncReadExt, BufReader, WriteHalf},
    AsyncWriteExt, SinkExt, StreamExt,
};
use serde_json::{Map, Value};
use stop_token::future::FutureExt as stopFutureExt;
use tracing::{trace, warn};

pub async fn upstream_message_handler<
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
>(
    config: UpstreamConfig,
    upstream_router: Arc<Router<State, CState>>,
    urx: UnboundedReceiver<String>,
    state: State,
    connection: Arc<Connection<CState>>,
    mut urtx: UnboundedSender<Map<String, Value>>,
    global_vars: GlobalVars,
) -> Result<()> {
    if config.enabled {
        let upstream = TcpStream::connect(config.url).await?;

        let (urh, uwh) = upstream.split();
        let mut upstream_buffer_stream = BufReader::new(urh);

        async_std::task::spawn(async move {
            match upstream_send_loop(urx, uwh).await {
                //@todo not sure if we even want a info here, we need an ID tho.
                Ok(_) => trace!("Upstream Send Loop is closing for connection"),
                Err(e) => warn!(
                    "Upstream Send loop is closed for connection: {}, Reason: {}",
                    1, e
                ),
            }
        });

        async_std::task::spawn({
            let state = state.clone();
            let connection = connection.clone();
            let stop_token = connection.get_stop_token();

            async move {
                loop {
                    // @todo actually think about a real timeout here as well.
                    let next_message =
                        next_message(&mut upstream_buffer_stream).timeout_at(stop_token.clone());

                    let (method, values) = match next_message.await? {
                        Ok(mv) => mv,
                        Err(_) => {
                            break;
                        }
                    };

                    if method == "result" {
                        if let MessageValue::StratumV1(map) = values {
                            urtx.send(map).await?;
                        }
                        continue;
                    }

                    upstream_router
                        .call(
                            &method,
                            values,
                            state.clone(),
                            connection.clone(),
                            global_vars.clone(),
                        )
                        .await;
                }
                Ok::<(), Error>(())
            }
        });
    }
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
