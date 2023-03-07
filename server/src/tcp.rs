// #[cfg(feature = "upstream")]
// use {crate::config::UpstreamConfig, crate::upstream::upstream_message_handler};

use crate::{
    id_manager::IDManager,
    router::Router,
    session::Session,
    types::{ConnectionID, GlobalVars},
    BanManager, ConfigManager, Connection, Result, SessionList,
};
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, trace, warn};

//@todo finish up the logging in this

pub(crate) struct Handler<State, CState>
where
    CState: Send + Sync + Clone + 'static,
{
    //No Cleanup needed
    pub(crate) id: ConnectionID,
    pub(crate) ban_manager: BanManager,
    pub(crate) id_manager: IDManager,
    pub(crate) session_list: SessionList<CState>,
    pub(crate) config_manager: ConfigManager,

    // Not sure, but should test
    pub(crate) router: Arc<Router<State, CState>>,
    pub(crate) state: State,
    pub(crate) connection_state: CState,

    // Cleanup needed
    pub(crate) connection: Connection,
    pub(crate) cancel_token: CancellationToken,
    pub(crate) global_vars: GlobalVars,
}

impl<State: Clone + Send + Sync + 'static, CState: Default + Clone + Send + Sync + 'static>
    Handler<State, CState>
{
    pub(crate) async fn run(mut self) -> Result<()> {
        let address = if self.config_manager.proxy_protocol() {
            self.connection.proxy_protocol().await?
        } else {
            self.connection.address
        };

        self.ban_manager.check_banned(address)?;

        let (mut reader, tx, handle) = self.connection.init();

        let session = Session::new(
            self.id.clone(),
            self.id_manager.clone(),
            tx,
            self.config_manager.clone(),
            self.cancel_token.child_token(),
            self.connection_state.clone(),
        )?;

        debug!(
            id = ?self.id,
            ip = &address.to_string(),
            "Connection initialized",
        );

        self.session_list.add_miner(address, session.clone());

        while !self.cancel_token.is_cancelled() {
            if session.is_disconnected() {
                trace!(
                    "Session: {} disconnected. Breaking out of next_message loop",
                    session.id()
                );
                break;
            }

            //@todo we need a timeout here otherwise we can get stuck forever.
            let maybe_frame = tokio::select! {
                res = reader.read_frame() => {
                    match res {
                        Err(e) => {
                            warn!("Session: {} errored with the following error: {}", session.id(), e);
                            break;
                        },
                        Ok(frame) => frame,
                    }
                },
                _ = self.cancel_token.cancelled() => {
                    // If a shutdown signal is received, return from `run`.
                    // This will result in the task terminating.
                    break;
                }
            };

            let Some(frame) = maybe_frame else {
                break;
            };

            //Calls the Stratum method on the router.
            self.router
                .call(
                    frame,
                    self.state.clone(),
                    session.clone(),
                    self.global_vars.clone(),
                )
                .await;
        }

        trace!(
            id = &self.id.to_string(),
            ip = &address.to_string(),
            "Connection shutdown started",
        );

        self.session_list.remove_miner(address);

        if session.needs_ban() {
            self.ban_manager.add_ban(address);
        }

        session.shutdown();

        //@todo below comment for older code, not accurate - review this though please.
        //@todo swap this to self.cancel_token.cancel() and don't return this from the function. Should
        //have the same effect
        self.cancel_token.cancel();

        //@todo we should also have a timeout here - but I may change write loop so we'll see
        if let Err(e) = handle.await {
            trace!(id = ?self.id, cause = ?e, "Write loop error");
        }

        trace!(
            id = ?self.id,
            ip = &address.to_string(),
            "Connection shutdown complete",
        );

        Ok(())
    }
}

//@todo I big think that I think we need to focus on today is catching attacks like open sockets
//doing nothing, socketrs trying to flood, etc.
//Let's make sure we have an entire folder of tests for "attacks" and make sure that we cover them
//thoroughly.

//     //@todo handle this undwrap?
//     connection_list
//         .add_miner(addr, connection.clone())
//         .await
//         .unwrap();
//
//     loop {
//         //@todo we could possibly do something like Cancellation token and see miniRedis from tokyo
//         //where this would be a future, so we can put it into select!
//         if connection.is_disconnected().await {
//             trace!(
//                 "Connection: {} disconnected. Breaking out of next_message loop",
//                 connection.id()
//             );
//             break;
//         }
//
//         let timeout = connection.timeout().await;
//
//         tokio::select! {
//         //@todo try this suggestion later.
//         //If this returns first, it's either a Timeout, or successful message read.
//         //We should also try the "else" method here so we would match Ok(msg) = and then cancel
//         //match, and then the else would be a timeout message which would match Err(msg)
//         res = tokio::time::timeout(timeout, next_message(&mut buffer_stream)) => {
//
//                     //Next_message Success
//                     if let Ok(result) = res {
//                     match result {
//                     Ok((method, values)) => {
//                         router
//                             .call(
//                                 &method,
//                                 values,
//                                 state.clone(),
//                                 connection.clone(),
//                                 global_vars.clone(),
//                             )
//                             .await;
//                     },
//                     Err(e) => {
//                         error!(
//                             "Connection: {} error in 'next_message' (decoding/reading) Error: {}",
//                             connection.id(), e
//                         );
//                         break;
//                 }
//
//                     }
//
//                     } else {
//             error!(connection_id=connection.id().to_string(), timeout=timeout.as_secs(), "next_message timed out.");
//
//                 }
//             }
//         _ = cancel_token.cancelled() => {
//             error!(connection_id=connection.id().to_string(), "Message parsing canceled. Received Shutdown");
//             break;
//         }
//
//         }
//     }
//
//     // let next_message = tokio::time::timeout(timeout, next_message(&mut buffer_stream))
//     //     .await;
//     //
//     // match next_message {
//     //     //@todo this would most likely be stop_token
//     //     Err(e) => tracing::error!(
//     //         "Connection: {} error in 'next_message' (stop_token) Error: {}",
//     //         connection.id(),
//     //         e
//     //     ),
//     //     Ok(msg) => {
//     //         //@todo this would most likely be timeout function
//     //         match msg {
//     //             Err(e) => {
//     //                 tracing::error!(
//     //                     "Connection: {} error in 'next_message' (timeout fn) Error: {}",
//     //                     connection.id(),
//     //                     e
//     //                 );
//     //                 break;
//     //             }
//     //             Ok(msg) => match msg {
//     //                 Err(e) => {
//     //                     tracing::error!(
//     //                         "Connection: {} error in 'next_message' (decoding/reading) Error: {}",
//     //                         connection.id(), e
//     //                     );
//     //                     break;
//     //                 }
//     //                 Ok((method, values)) => {
//     //                     router
//     //                         .call(
//     //                             &method,
//     //                             values,
//     //                             state.clone(),
//     //                             connection.clone(),
//     //                             global_vars.clone(),
//     //                         )
//     //                         .await;
//     //                 }
//     //             },
//     //         }
//     //     }
//     // }
//
//     //@todo maybe do triple ??? instead?
//     //@todo I don't think we like the triple ??? actually because we want to break the loop and
//     //not automatically complete the function so we can do shutdown proceedures.
//     //Check to see if we did ? anywhere, and if so let's fix that.
//     // if let Ok(Ok(Ok((method, values)))) = next_message {
//     //     router
//     //         .call(
//     //             &method,
//     //             values,
//     //             state.clone(),
//     //             connection.clone(),
//     //             global_vars.clone(),
//     //         )
//     //         .await;
//     // } else {
//     //     break;
//     // }
//
//     //@todo on that note, let's go through this workflow as if we are a complete hack and see if we
//     //can figure out if there are any bad spots.
//     //Not necessarily a hack, but say like a random request from a random website.
//     trace!("Closing stream from: {}", connection.id());
//
//     connection_list.remove_miner(addr).await;
//
//     if connection.needs_ban().await {
//         ban_manager.add_ban(addr);
//     }
//
//     connection.shutdown().await;
//
//     Ok(())
// }
