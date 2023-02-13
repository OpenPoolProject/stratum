use crate::{
    config::ConfigManager, format_difficulty, id_manager::IDManager, Miner, MinerList, Result,
};
use extended_primitives::Buffer;
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Duration, SystemTime},
};
use tokio::sync::{mpsc::UnboundedSender, Mutex, RwLock};
use tokio_util::sync::CancellationToken;
use tracing::{debug, trace};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct UserInfo {
    pub account_id: i32,
    pub mining_account: i32,
    pub worker_name: Option<String>,
}

//@todo remove
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct SessionInfo {
    pub agent: bool,
    pub authorized: bool,
    pub subscribed: bool,
    pub client: Option<String>,
    pub session_start: SystemTime,
    pub state: SessionState,
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
            state: SessionState::Connected,
            is_long_timeout: false,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub enum SessionState {
    Connected,
    Disconnect,
}

#[derive(Debug)]
pub enum SendInformation {
    Json(String),
    Text(String),
    Raw(Buffer),
}

//@todo we are going to rename this to StratumSession

//@todo thought process -> Rather than have this boolean variables that slowly add up over time, we
//should add a new type of "SessionType". This will allow us to also incorporate other types of
//connections that are developed in the future or that are already here and enables a lot easier
//pattern matching imo.
//
//@todo also think about these enums for connectgion sttatus like authenticated/subscribed etc.
pub struct Session<State> {
    pub id: Uuid,
    pub session_id: u32,
    pub id_manager: IDManager,

    pub info: Arc<RwLock<SessionInfo>>,
    pub user_info: Arc<Mutex<UserInfo>>,

    pub sender: Arc<Mutex<UnboundedSender<SendInformation>>>,
    // #[cfg(feature = "upstream")]
    // pub upstream_sender: Arc<Mutex<UnboundedSender<String>>>,
    // #[cfg(feature = "upstream")]
    // pub upstream_receiver:
    //     Arc<Mutex<UnboundedReceiver<serde_json::map::Map<String, serde_json::Value>>>>,

    //@todo one thing we could do here that I luike quite a bit is to just make this a tuple.
    //(old, new)
    pub difficulty: Arc<Mutex<u64>>,
    pub previous_difficulty: Arc<Mutex<u64>>,
    pub next_difficulty: Arc<Mutex<Option<u64>>>,
    pub options: Arc<MinerOptions>,

    pub needs_ban: Arc<Mutex<bool>>,
    pub state: Arc<Mutex<State>>,
    pub cancel_token: CancellationToken,

    //@todo probably redo this quite a bit but for now it works.
    //@todo if we make Miner send/safe etc then we can rmeove all of the Arc shit.
    pub connection_miner: Arc<Mutex<Option<Miner>>>,
    pub miner_list: MinerList,
}

//@todo this should probably come from builder pattern
//@todo just turn this into VardiffConfig
#[derive(Debug, Default)]
pub struct MinerOptions {
    //@todo this is in seconds.
    pub retarget_time: u64, //300 Seconds
    pub target_time: u64,   //10 seconds
    pub min_diff: u64,
    pub max_diff: u64,
    pub max_delta: f64,
    pub variance_percent: f64,
    // share_time_min: f64,
    // share_time_max: f64,
}

impl<State: Clone + Send + Sync + 'static> Session<State> {
    //@todo remove this if/when we
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        id_manager: IDManager,
        sender: UnboundedSender<SendInformation>,
        config_manager: ConfigManager,
        cancel_token: CancellationToken,
        // #[cfg(feature = "upstream")] upstream_sender: UnboundedSender<String>,
        // #[cfg(feature = "upstream")] upstream_receiver: UnboundedReceiver<
        //     serde_json::map::Map<String, serde_json::Value>,
        // >,
        state: State,
    ) -> Result<Self> {
        let session_id = id_manager.allocate_session_id()?;
        let id = Uuid::new_v4();

        debug!("Accepting new miner. ID: {}", &id);

        let config = config_manager.current_config();

        let options = MinerOptions {
            retarget_time: config.difficulty.retarget_time,
            target_time: config.difficulty.target_time,
            //@todo these values make no sense so let's trim them a bit.
            min_diff: config.difficulty.minimum_difficulty,
            max_diff: config.difficulty.maximum_difficulty,
            max_delta: 1.0, //@todo make this adjustable, not sure if this is solid or not.
            //@todo probably don't store, get from above and then calcualte the others.
            variance_percent: config.difficulty.variance_percent,
            // share_time_min: 4.2,
            // share_time_max: 7.8,
        };

        Ok(Session {
            id,
            session_id,
            id_manager,
            user_info: Arc::new(Mutex::new(UserInfo {
                account_id: 0,
                mining_account: 0,
                worker_name: None,
            })),
            info: Arc::new(RwLock::new(SessionInfo::new())),
            sender: Arc::new(Mutex::new(sender)),
            // #[cfg(feature = "upstream")]
            // upstream_sender: Arc::new(Mutex::new(upstream_sender)),
            // #[cfg(feature = "upstream")]
            // upstream_receiver: Arc::new(Mutex::new(upstream_receiver)),
            difficulty: Arc::new(Mutex::new(config.difficulty.initial_difficulty)),
            previous_difficulty: Arc::new(Mutex::new(config.difficulty.initial_difficulty)),
            next_difficulty: Arc::new(Mutex::new(None)),
            options: Arc::new(options),
            needs_ban: Arc::new(Mutex::new(false)),
            state: Arc::new(Mutex::new(state)),
            cancel_token,
            miner_list: MinerList::new(),
            connection_miner: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn is_disconnected(&self) -> bool {
        self.info.read().await.state == SessionState::Disconnect
    }

    //@todo we have disabled last_active for now... Need to reimplement this desperately.
    pub async fn send<T: Serialize>(&self, message: T) -> Result<()> {
        //let last_active = self.stats.lock().await.last_active;

        //let last_active_ago = Utc::now().naive_utc() - last_active;

        ////@todo rewrite this comment
        ////If the miner has not been active (sending shares) for 5 minutes, we disconnect this dude.
        ////@todo before live, check this guy. Also should come from options.
        ////@todo make the last_active thing a config.
        //if last_active_ago > Duration::seconds(600) {
        //    warn!(
        //        "Miner: {} not active since {}. Disconnecting",
        //        self.id, last_active
        //    );
        //    self.ban().await;
        //    return Ok(());
        //}

        let msg_string = serde_json::to_string(&message)?;

        trace!("Sending message: {}", msg_string.clone());

        let sender = self.sender.lock().await;

        //@todo this feels inefficient, maybe we do send bytes here.
        sender.send(SendInformation::Json(msg_string))?;
        // stream.write_all(b"\n").await?;

        Ok(())
    }

    pub async fn send_text(&self, message: String) -> Result<()> {
        let sender = self.sender.lock().await;

        sender.send(SendInformation::Text(message))?;

        Ok(())
    }

    pub async fn send_raw(&self, message: Buffer) -> Result<()> {
        let sender = self.sender.lock().await;

        sender.send(SendInformation::Raw(message))?;

        Ok(())
    }

    // #[cfg(feature = "upstream")]
    // pub async fn upstream_send<T: Serialize>(&self, message: T) -> Result<()> {
    //     //let last_active = self.stats.lock().await.last_active;
    //
    //     //let last_active_ago = Utc::now().naive_utc() - last_active;
    //
    //     ////@todo rewrite this comment
    //     ////If the miner has not been active (sending shares) for 5 minutes, we disconnect this dude.
    //     ////@todo before live, check this guy. Also should come from options.
    //     ////@todo make the last_active thing a config.
    //     //if last_active_ago > Duration::seconds(600) {
    //     //    warn!(
    //     //        "Miner: {} not active since {}. Disconnecting",
    //     //        self.id, last_active
    //     //    );
    //     //    self.ban().await;
    //     //    return Ok(());
    //     //}
    //
    //     let msg_string = serde_json::to_string(&message)?;
    //
    //     debug!("Sending message: {}", msg_string.clone());
    //
    //     let mut upstream_sender = self.upstream_sender.lock().await;
    //
    //     //@todo this feels inefficient, maybe we do send bytes here.
    //     upstream_sender.send(msg_string).await?;
    //     // stream.write_all(b"\n").await?;
    //
    //     Ok(())
    // }

    // #[cfg(feature = "upstream")]
    // pub async fn upstream_result(&self) -> Result<(serde_json::Value, serde_json::Value)> {
    //     let mut upstream_receiver = self.upstream_receiver.lock().await;
    //
    //     let values = match upstream_receiver.next().await {
    //         Some(values) => values,
    //         //@todo return error here.
    //         None => return Ok((json!(false), serde_json::Value::Null)),
    //     };
    //
    //     Ok((values["result"].clone(), values["error"].clone()))
    // }

    pub async fn shutdown(&self) {
        self.info.write().await.state = SessionState::Disconnect;

        self.cancel_token.cancel();
    }

    pub async fn disconnect(&self) {
        self.info.write().await.state = SessionState::Disconnect;
    }

    pub async fn ban(&self) {
        *self.needs_ban.lock().await = true;
        self.disconnect().await;
    }

    pub async fn needs_ban(&self) -> bool {
        *self.needs_ban.lock().await
    }

    #[must_use]
    pub fn id(&self) -> Uuid {
        self.id
    }

    pub async fn add_main_worker(&self, worker_id: Uuid) {
        let conn_info = self.get_connection_info().await;
        let user_info = self.get_user_info().await;
        let session_id = self.session_id;

        let worker = Miner::new(
            worker_id,
            conn_info.client.clone(),
            user_info.worker_name.clone(),
            Buffer::from(session_id.to_le_bytes().to_vec()),
            self.options.clone(),
            format_difficulty(*self.difficulty.lock().await),
        );

        *self.connection_miner.lock().await = Some(worker);
    }

    pub async fn get_main_worker(&self) -> Option<Miner> {
        self.connection_miner.lock().await.clone()
    }

    pub async fn register_worker(
        &self,
        session_id: u32,
        client_agent: &str,
        worker_name: &str,
        worker_id: Uuid,
    ) {
        let worker = Miner::new(
            worker_id,
            Some(client_agent.to_owned()),
            Some(worker_name.to_owned()),
            Buffer::from(session_id.to_le_bytes().to_vec()),
            self.options.clone(),
            format_difficulty(*self.difficulty.lock().await),
        );

        self.miner_list.add_miner(session_id, worker);
    }

    //@todo make session_id a custom type.
    #[must_use]
    pub fn unregister_worker(&self, session_id: u32) -> Option<(u32, Miner)> {
        self.miner_list.remove_miner(session_id)
    }

    #[must_use]
    pub fn get_worker_list(&self) -> MinerList {
        self.miner_list.clone()
    }

    #[must_use]
    pub fn get_worker_by_session_id(&self, session_id: u32) -> Option<Miner> {
        self.miner_list.get_miner_by_id(session_id)
    }

    pub fn update_worker_by_session_id(&self, session_id: u32, miner: Miner) {
        self.miner_list
            .update_miner_by_session_id(session_id, miner);
    }

    // ===== Worker Helper functions ===== //
    pub async fn set_user_info(
        &self,
        account_id: i32,
        mining_account_id: i32,
        worker_name: Option<String>,
    ) {
        let mut user_info = self.user_info.lock().await;
        user_info.account_id = account_id;
        user_info.mining_account = mining_account_id;
        //@tood idk if we need this here actually.
        user_info.worker_name = worker_name;
    }

    pub async fn get_user_info(&self) -> UserInfo {
        self.user_info.lock().await.clone()
    }

    pub async fn set_client(&self, client: &str) {
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

        let mut info = self.info.write().await;
        info.agent = agent;
        info.client = Some(client.to_string());
        info.is_long_timeout = long_timeout;
    }

    pub async fn get_connection_info(&self) -> SessionInfo {
        self.info.read().await.clone()
    }

    pub async fn is_long_timeout(&self) -> bool {
        self.info.read().await.is_long_timeout
    }

    // Returns the current timeout
    pub async fn timeout(&self) -> Duration {
        let info = self.info.read().await;

        if info.is_long_timeout {
            // One Week
            Duration::from_secs(86400 * 7)
        } else if info.subscribed && info.authorized {
            // Ten Minutes
            Duration::from_secs(600)
        } else {
            //@todo let's play with this -> I think 15 might be too short, but if it works lets do
            //it.
            Duration::from_secs(15)
        }
    }

    #[must_use]
    pub fn get_session_id(&self) -> u32 {
        self.session_id
    }

    pub async fn authorized(&self) -> bool {
        self.info.read().await.authorized
    }

    pub async fn authorize(&self) {
        self.info.write().await.authorized = true;
    }

    pub async fn subscribed(&self) -> bool {
        self.info.read().await.subscribed
    }

    pub async fn subscribe(&self) {
        self.info.write().await.subscribed = true;
    }

    pub async fn is_agent(&self) -> bool {
        self.info.read().await.agent
    }

    pub async fn set_difficulty(&self, difficulty: u64) {
        let miner = self.connection_miner.lock().await.clone();

        if let Some(connection_miner) = miner {
            connection_miner
                .set_difficulty(format_difficulty(difficulty))
                .await;
        } else {
            //If the miner is not set yet, we save it in the connection, and the miner will
            //draw it from there.
            *self.difficulty.lock().await = format_difficulty(difficulty);
        }
    }

    pub async fn get_difficulty(&self) -> u64 {
        let miner = self.connection_miner.lock().await.clone();

        if let Some(connection_miner) = miner {
            connection_miner.current_difficulty().await
        } else {
            *self.difficulty.lock().await
        }
    }

    pub async fn get_previous_difficulty(&self) -> u64 {
        let miner = self.connection_miner.lock().await.clone();

        if let Some(connection_miner) = miner {
            connection_miner.previous_difficulty().await
        } else {
            *self.previous_difficulty.lock().await
        }
    }

    pub async fn get_state(&self) -> State {
        self.state.lock().await.clone()
    }

    pub async fn set_state(&self, state: State) {
        *self.state.lock().await = state;
    }

    pub async fn update_difficulty(&self) -> Option<u64> {
        let miner = self.connection_miner.lock().await.clone();

        if let Some(connection_miner) = miner {
            connection_miner.update_difficulty().await
        } else {
            None
        }
    }

    // pub(crate) fn get_cancel_token(&self) -> CancellationToken {
    //     self.cancel_token.child_token()
    // }
}

impl<State> Drop for Session<State> {
    fn drop(&mut self) {
        self.id_manager.remove_session_id(self.session_id);
    }
}
