use crate::{
    id_manager::IDManager,
    router::Router,
    session::Session,
    types::{ConnectionID, GlobalVars},
    BanManager, ConfigManager, Connection, Result, SessionList,
};
use std::sync::Arc;
use tokio::time::{sleep, Duration, Instant};
use tokio_util::sync::CancellationToken;
use tracing::{enabled, error, trace, warn, Level};

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

        if self.config_manager.ban_manager_enabled() {
            self.ban_manager.check_banned(address)?;
        }

        let (mut reader, tx, handle) = self.connection.init();

        let session_id = self.id_manager.allocate_session_id()?;

        let session_cancel_token = self.cancel_token.child_token();

        let session = Session::new(
            self.id.clone(),
            session_id,
            address,
            tx,
            self.config_manager.clone(),
            session_cancel_token.clone(),
            self.connection_state,
        )?;

        trace!(
            id = ?self.id,
            ip = &address.to_string(),
            "Connection initialized",
        );

        self.session_list.add_miner(address, session.clone());

        //@todo mark this somewhere as the default timeout
        let sleep = sleep(Duration::from_secs(15));
        tokio::pin!(sleep);

        //@todo we can return a value from this loop -> break can return a value, and so we may
        //want to return an error if there is one so that we can report it at the end.
        while !self.cancel_token.is_cancelled() {
            if session.is_disconnected() {
                trace!( id = ?self.id, ip = &address.to_string(), "Session disconnected.");
                break;
            }

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
                        () = &mut sleep => {
            if enabled!(Level::DEBUG) {
                error!( id = &self.id.to_string(), ip = &address.to_string(), "Session Parse Frame Timeout");
            }
                    break;
                },
                        //@todo we might want timeouts to reduce difficulty as well here. -> That is
                        //handled in retarget, so let's check that out.
                    () = session_cancel_token.cancelled() => {
                        //@todo work on these errors,
            if enabled!(Level::DEBUG) {
                error!( id = &self.id.to_string(), ip = &address.to_string(), "Session Disconnected");
            }
                        break;
                    },
                    () = self.cancel_token.cancelled() => {
                        // If a shutdown signal is received, return from `run`.
                        // This will result in the task terminating.
                        break;
                    }
                };

            let Some(frame) = maybe_frame else {
                break;
            };

            //Resets the Session's last active, to detect for unactive connections
            session.active();

            //@todo if a miner fails a function, like subscribe / authorize we don't catch it, and
            //they can spam us.
            //Calls the Stratum method on the router.
            self.router
                .call(
                    frame,
                    self.state.clone(),
                    //@todo would it be possible to pass session by reference?
                    session.clone(),
                    self.global_vars.clone(),
                )
                .await;

            //Reset sleep as later as possible
            sleep.as_mut().reset(Instant::now() + session.timeout());
        }

        trace!(
            id = &self.id.to_string(),
            ip = &address.to_string(),
            "Connection shutdown started",
        );

        self.session_list.remove_miner(address);
        self.id_manager.remove_session_id(session_id);

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
