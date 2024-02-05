use crate::{
    config::ConfigManager,
    types::{ConnectionID, Difficulties, Difficulty, DifficultySettings},
    Miner, MinerList, Result, SessionID,
};
use extended_primitives::Buffer;
use parking_lot::{Mutex, RwLock};
use serde::Serialize;
use std::{
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error};
use uuid::Uuid;

//@todo remove this excessive_bools
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub agent: bool,
    pub authorized: bool,
    pub subscribed: bool,
    pub client: Option<String>,
    pub session_start: SystemTime,
    // pub state: SessionState,
    pub is_long_timeout: bool,
}

impl Default for SessionInfo {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionInfo {
    pub fn new() -> Self {
        SessionInfo {
            agent: false,
            authorized: false,
            subscribed: false,
            client: None,
            session_start: SystemTime::now(),
            // state: SessionState::Connected,
            is_long_timeout: false,
        }
    }
}

#[derive(PartialEq, Eq, Debug)]
pub enum SessionState {
    Connected,
    Disconnected,
}

#[derive(Debug)]
pub enum SendInformation {
    Json(String),
    Text(String),
    Raw(Buffer),
}

//@todo thought process -> Rather than have this boolean variables that slowly add up over time, we
//should add a new type of "SessionType". This will allow us to also incorporate other types of
//connections that are developed in the future or that are already here and enables a lot easier
//pattern matching imo.

//@todo also think about these enums for connectgion sttatus like authenticated/subscribed etc.
#[derive(Clone)]
pub struct Session<State> {
    inner: Arc<Inner<State>>,
    config_manager: ConfigManager,

    cancel_token: CancellationToken,
    miner_list: MinerList,
    shared: Arc<Mutex<Shared>>,
    difficulty_settings: Arc<RwLock<DifficultySettings>>,
}

//@todo I want to get IP here and return it
struct Inner<State> {
    pub id: ConnectionID,
    pub session_id: SessionID,
    pub ip: SocketAddr,
    pub state: State,
}

//@todo I think we need to have a few more Mutex's here otherwise we run the risk of deadlocks.
pub(crate) struct Shared {
    //@todo possibly turn this into an Atomic
    status: SessionState,
    //@todo change this (But Later)
    sender: UnboundedSender<SendInformation>,
    needs_ban: bool,
    last_active: Instant,
    //@todo wrap this in a RwLock I believe
    info: SessionInfo,
}

impl<State: Clone> Session<State> {
    pub fn new(
        id: ConnectionID,
        session_id: SessionID,
        ip: SocketAddr,
        sender: UnboundedSender<SendInformation>,
        config_manager: ConfigManager,
        cancel_token: CancellationToken,
        state: State,
    ) -> Result<Self> {
        let config = config_manager.current_config();

        let shared = Shared {
            status: SessionState::Connected,
            last_active: Instant::now(),
            needs_ban: false,
            sender,
            info: SessionInfo::new(),
        };

        let inner = Inner {
            id,
            session_id,
            ip,
            state,
        };

        Ok(Session {
            config_manager,
            cancel_token,
            miner_list: MinerList::new(),
            shared: Arc::new(Mutex::new(shared)),
            inner: Arc::new(inner),
            difficulty_settings: Arc::new(RwLock::new(DifficultySettings {
                default: Difficulty::from(config.difficulty.initial_difficulty),
                minimum: Difficulty::from(config.difficulty.minimum_difficulty),
            })),
        })
    }

    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.shared.lock().status == SessionState::Disconnected
    }

    pub fn send<T: Serialize>(&self, message: T) -> Result<()> {
        let shared = self.shared.lock();

        if shared.last_active.elapsed()
            > Duration::from_secs(
                self.config_manager
                    .current_config()
                    .connection
                    .active_timeout,
            )
        {
            error!(
                "Session: {} not active for 10 minutes. Disconnecting",
                self.inner.id,
            );
            drop(shared);

            self.ban();

            //@todo return Error here instead
            return Ok(());
        }

        debug!("Sending message: {}", serde_json::to_string(&message)?);

        //@todo implement Display on SendInformation.
        let msg = SendInformation::Json(serde_json::to_string(&message)?);

        //@todo it may make sense to keep the sender inside of session here - not sure why it's in
        //connection like the way it is.
        //@todo this feels inefficient, maybe we do send bytes here.
        shared.sender.send(msg)?;

        Ok(())
    }

    pub fn send_raw(&self, message: Buffer) -> Result<()> {
        let shared = self.shared.lock();

        shared.sender.send(SendInformation::Raw(message))?;

        Ok(())
    }

    pub fn shutdown(&self) {
        if !self.is_disconnected() {
            self.disconnect();

            self.cancel_token.cancel();
        }
    }

    pub fn disconnect(&self) {
        self.shared.lock().status = SessionState::Disconnected;
    }

    pub fn ban(&self) {
        self.shared.lock().needs_ban = true;
        self.shutdown();
    }

    #[must_use]
    pub fn needs_ban(&self) -> bool {
        self.shared.lock().needs_ban
    }

    #[must_use]
    pub fn id(&self) -> &ConnectionID {
        &self.inner.id
    }

    pub fn register_worker(
        &self,
        session_id: SessionID,
        client: Option<String>,
        worker_name: Option<String>,
        worker_id: Uuid,
    ) {
        //@todo has to be an easier way to reuse worker_name here
        debug!(id = ?self.inner.id, "Registered Worker {worker_id} ({}) Session ID: {session_id}", worker_name.clone().unwrap_or_default());

        let worker = Miner::new(
            self.id().clone(),
            worker_id,
            session_id,
            client,
            worker_name,
            self.config_manager.clone(),
            self.difficulty_settings.read().clone(),
        );

        self.miner_list.add_miner(session_id, worker);
    }

    #[must_use]
    pub fn unregister_worker(&self, session_id: SessionID) -> Option<(SessionID, Miner)> {
        self.miner_list.remove_miner(session_id)
    }

    #[must_use]
    pub fn get_miner_list(&self) -> MinerList {
        self.miner_list.clone()
    }

    #[must_use]
    pub fn get_worker_by_session_id(&self, session_id: SessionID) -> Option<Miner> {
        self.miner_list.get_miner_by_id(session_id)
    }

    //@todo I think we can remove this
    pub fn update_worker_by_session_id(&self, session_id: SessionID, miner: Miner) {
        self.miner_list
            .update_miner_by_session_id(session_id, miner);
    }

    // ===== Worker Helper functions ===== //

    pub fn set_client(&self, client: &str) {
        let mut agent = false;
        let mut long_timeout = false;
        //@todo check these equal to just STATIC CONSTS for the various things we need to know.
        //ClientType Enum
        //@todo we need to do some checking/pruning etc of this client string.

        if client.starts_with("btccom-agent/") {
            //Agent
            agent = true;
            long_timeout = true;
        }

        let mut shared = self.shared.lock();
        shared.info.agent = agent;
        shared.info.client = Some(client.to_string());
        shared.info.is_long_timeout = long_timeout;
    }

    #[must_use]
    pub fn get_connection_info(&self) -> SessionInfo {
        self.shared.lock().info.clone()
    }

    #[must_use]
    pub fn is_long_timeout(&self) -> bool {
        self.shared.lock().info.is_long_timeout
    }

    // Returns the current timeout
    #[must_use]
    pub fn timeout(&self) -> Duration {
        let shared = self.shared.lock();

        if shared.info.is_long_timeout {
            // One Week
            Duration::from_secs(86400 * 7)
        } else if shared.info.subscribed && shared.info.authorized {
            // Ten Minutes
            Duration::from_secs(600)
        } else {
            //@todo let's play with this -> I think 15 might be too short, but if it works lets do
            //it.
            Duration::from_secs(15)
        }
    }

    #[must_use]
    pub fn get_session_id(&self) -> SessionID {
        self.inner.session_id
    }

    #[must_use]
    pub fn authorized(&self) -> bool {
        self.shared.lock().info.authorized
    }

    pub fn authorize(&self) {
        self.shared.lock().info.authorized = true;
    }

    #[must_use]
    pub fn subscribed(&self) -> bool {
        self.shared.lock().info.subscribed
    }

    pub fn subscribe(&self) {
        self.shared.lock().info.subscribed = true;
    }

    #[must_use]
    pub fn is_agent(&self) -> bool {
        self.shared.lock().info.agent
    }

    pub fn set_difficulty(&self, session_id: SessionID, difficulty: Difficulty) {
        if let Some(miner) = self.miner_list.get_miner_by_id(session_id) {
            miner.set_difficulty(difficulty);
        }
    }

    pub fn set_default_difficulty(&self, difficulty: Difficulty) {
        self.difficulty_settings.write().default = difficulty;
    }

    //@todo we need to test this
    pub fn set_minimum_difficulty(&self, difficulty: Difficulty) {
        //We only want to set the minimum difficulty if it is greater than or equal to the Global
        //minimum difficulty
        if difficulty.as_u64() >= self.config_manager.difficulty_config().minimum_difficulty {
            self.difficulty_settings.write().minimum = difficulty;
        }
    }

    #[must_use]
    pub fn get_difficulties(&self, session_id: SessionID) -> Option<Difficulties> {
        self.miner_list
            .get_miner_by_id(session_id)
            .map(|miner| miner.difficulties())
    }

    //@todo double check that a reference here will work
    #[must_use]
    pub fn state(&self) -> &State {
        &self.inner.state
    }

    #[must_use]
    pub fn update_difficulty(&self, session_id: SessionID) -> Option<Difficulty> {
        if let Some(miner) = self.miner_list.get_miner_by_id(session_id) {
            miner.update_difficulty()
        } else {
            None
        }
    }

    pub(crate) fn active(&self) {
        self.shared.lock().last_active = Instant::now();
    }

    #[must_use]
    pub fn ip(&self) -> SocketAddr {
        self.inner.ip
    }

    //@todo Use internal URLs if they exist otherwise use the default URL?
    //Figure out how to do this gracefully.
    // pub(crate) fn graceful_shutdown(&self) {
    //     todo!()
    // }
}

#[cfg(feature = "test-utils")]
impl<State: Clone> Session<State> {
    pub fn mock(state: State) -> Session<State> {}
}
