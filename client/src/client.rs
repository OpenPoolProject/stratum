use crate::types::{Event, Message, State};
use async_std::net::TcpStream;
use async_std::sync::{Arc, Mutex};
use async_std::task;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::io::BufReader;
use futures::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use futures::io::{ReadHalf, WriteHalf};
use futures::sink::SinkExt;
use futures::StreamExt;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use stratum_types::params::{Params, Results, SetDiff};
use stratum_types::traits::{PoolParams, StratumParams, Subscribe, SubscribeResult};
use stratum_types::Error;
use stratum_types::Result;
use stratum_types::{Request, StratumError, StratumMethod, StratumPacket, ID};

#[derive(Clone, Debug, Deserialize)]
pub struct ClientConfig<PP>
where
    PP: PoolParams,
{
    pub host: String,
    pub credentials: PP::Authorize,
    pub subscriber_info: PP::Subscribe,
}

#[derive(Debug, Clone)]
pub struct StratumClient<PP, SP>
where
    PP: PoolParams,
    SP: StratumParams,
{
    event_rx: Arc<Mutex<UnboundedReceiver<Event>>>,
    event_tx: Arc<Mutex<UnboundedSender<Event>>>,
    message_tx: Arc<Mutex<UnboundedSender<Message<SP>>>>,
    message_rx: Arc<Mutex<UnboundedReceiver<Message<SP>>>>,
    pub write_half: Arc<Mutex<WriteHalf<TcpStream>>>,
    pub read_half: Arc<Mutex<BufReader<ReadHalf<TcpStream>>>>,
    pub config: Arc<ClientConfig<PP>>,
    state: Arc<Mutex<State>>,
    //@todo I think the name of Notify is sloppy. We should just name this "Job", and then have it
    //in the notify packet.
    //@todo maybe don't make this an option since after start it will never be none. Just make sure
    //that we can default it, and then use it.
    current_job: Arc<Mutex<Option<SP::Notify>>>,
    new_work: Arc<Mutex<bool>>,
    difficulty: Arc<Mutex<f64>>,
    subscriber_info: Arc<Mutex<PP::Subscribe>>,
    subscribe_id: Arc<Mutex<String>>,
}

//@todo build sync versions of all these functions as well. Then come clean it up.
impl<PP, SP> StratumClient<PP, SP>
where
    //@todo smells
    PP: PoolParams
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send
        + std::fmt::Debug
        + Clone
        + 'static,
    SP: StratumParams
        + Serialize
        + for<'de> Deserialize<'de>
        + Sync
        + Send
        + std::fmt::Debug
        + Clone
        + 'static,
{
    // ===== Constructor Methods ===== //
    pub async fn new(config: ClientConfig<PP>, sid: Option<String>) -> Result<Self> {
        let stream = TcpStream::connect(&config.host).await?;

        let (rh, wh) = stream.split();

        let (tx, rx) = unbounded();
        let (m_tx, m_rx) = unbounded();

        let subscribe_id = sid.unwrap_or("".to_owned());

        Ok(StratumClient {
            config: Arc::new(config.clone()),
            write_half: Arc::new(Mutex::new(wh)),
            read_half: Arc::new(Mutex::new(BufReader::new(rh))),
            state: Arc::new(Mutex::new(State::Connected)),
            event_tx: Arc::new(Mutex::new(tx)),
            event_rx: Arc::new(Mutex::new(rx)),
            message_rx: Arc::new(Mutex::new(m_rx)),
            message_tx: Arc::new(Mutex::new(m_tx)),
            current_job: Arc::new(Mutex::new(None)),
            new_work: Arc::new(Mutex::new(false)),
            difficulty: Arc::new(Mutex::new(0.0)),
            subscriber_info: Arc::new(Mutex::new(config.subscriber_info)),
            subscribe_id: Arc::new(Mutex::new(subscribe_id)),
        })
    }

    pub async fn init_connection(&self) -> Result<()> {
        *self.state.lock().await = State::Connected;
        if !self.authorize().await? {
            //handle bad auth
            return Ok(());
        }

        self.subscribe().await?;

        Ok(())
    }

    //@todo all the messages in here should be DEBUG.
    pub async fn init_server(&self) -> Result<()> {
        self.init_connection().await?;

        // Init the connection.
        // Start the Event loop for reading new messages (TO SEND).
        // Start the Read loop for incoming messages

        let send_self = self.clone();
        // let self_2 = send_self.clone();
        let sender_handle = task::spawn(async move {
            send_self.send_loop().await;
        });

        self.receive_loop().await;
        self.disconnect().await;
        //@todo not sure how we do this the best, but we want to wait for it to finish before
        //finishing the task here.
        //We whould be smart about which one ends first - so think if we want to stop receiving or
        //sending first and then write this accordingly.
        sender_handle.await;

        Ok(())
    }

    // ===== Start Methods ===== //

    //sync version of Start
    pub fn run(&self) -> Result<()> {
        async_std::task::block_on(self.start())
    }

    //@todo allow for multiple hostnames and a backoff function here.
    //So if the connection doesn't work rright away, we can exponentially backoff, and then switch
    //to a backup if even that doesn't work.
    //@todo this means we need to move the connection code into here.
    //Meat and potatoes function here.
    //You would spawn this function inside of a task.
    pub async fn start(&self) -> Result<()> {
        let mut retry_count = 0;
        let timeout = Duration::from_secs(10);

        //Atttempt to reconnect to stratum.
        loop {
            if retry_count > 0 {
                println!(
                    "Connection broken, attempting reconnect. Current attempts: {}",
                    retry_count
                );
            }
            self.init_server().await;

            println!("Connection broken, attempting to reconnect");

            let result = TcpStream::connect(&self.config.host).await;

            if let Ok(stream) = result {
                println!("Connection Reestablished. Continuing...");

                let (rh, wh) = stream.split();
                *self.write_half.lock().await = wh;
                *self.read_half.lock().await = BufReader::new(rh);
                println!("Initializating the server");
                self.init_server().await;
            } else {
                if retry_count > 12 {
                    break;
                }
                async_std::task::sleep(timeout).await;
                retry_count += 1;
            }
        }

        Ok(())
    }

    // ===== Loops ===== //

    //Loops on new messages coming in from the TcpStream.
    async fn receive_loop(&self) -> Result<()> {
        loop {
            if self.check_disconnect().await {
                break;
            }

            let msg = self.next_message().await?;

            match msg {
                StratumPacket::Request(req) => self.handle_requests(req).await?,
                StratumPacket::Response(res) => {
                    if let Some(result) = res.result {
                        self.handle_responses(result).await?;
                    } else if let Some(error) = res.error {
                        self.handle_errors(error).await?;
                    } else {
                        //Packet is null for both result and error. Not sure how to handle
                        //that.
                    }
                }
            };
        }

        Ok(())
    }

    async fn handle_requests(&self, req: Request<PP, SP>) -> Result<()> {
        match req.params {
            //@todo I think this needs to be in responses.
            // Params::SubmitResult(accepted) => self.handle_submit_result(accepted).await?,
            Params::Notify(job) => self.handle_notify(job).await?,
            Params::SetDifficulty(diff) => self.handle_difficulty(diff.difficulty).await?,
            _ => {
                // warn!("Invalid Message");
                dbg!("Got an invaid msg");
                //throw error
                // break;
            }
        };
        Ok(())
    }

    async fn handle_responses(&self, result: Results<PP, SP>) -> Result<()> {
        //@todo this can be removed, since we now handle it in the authorize function.
        if let PoolParam::AuthorizeResult(auth) = &result {
            self.handle_authorize_return(auth).await?;
            return Ok(());
        }

        match result {
            PoolParam::SubscribeResult(info) => self.handle_subscribe_result(info).await?,
            PoolParam::SubmitResult(_) => println!("Share Accepted!"),
            _ => {
                println!("Recieved unknown packet");
            }
        }

        Ok(())
    }

    async fn handle_errors(&self, error: StratumError) -> Result<()> {
        println!("Error: {}", error.to_string());
        // match error {
        //     StratumError::StaleShare => println!("Stratum - Error: Sent Stale Share"),
        //     _ => println!("Stratum - Error not handled yet"),
        // }

        Ok(())
    }

    //Loops on messages submitted for sending.
    async fn send_loop(&self) -> Result<()> {
        loop {
            if self.check_disconnect().await {
                break;
            }

            while let Some(msg) = self.message_rx.lock().await.next().await {
                match msg {
                    Message::Submit(share) => self.handle_submit_message(share).await?,
                }
            }
        }

        Ok(())
    }

    // ===== Helper Functions ===== //
    async fn check_disconnect(&self) -> bool {
        *self.state.lock().await == State::Disconnect
    }

    async fn disconnect(&self) {
        *self.state.lock().await = State::Disconnect;
    }

    //Parses in the incoming messages.
    pub async fn next_message(&self) -> Result<StratumPacket<PP, SP>> {
        loop {
            let mut stream = self.read_half.lock().await;

            let mut buf = String::new();

            //If this returns Ok(0), need to try to reconnect.
            let amt = stream.read_line(&mut buf).await?;

            if amt == 0 {
                return Err(Error::StreamClosed);
            }

            buf = buf.trim().to_string();

            if !buf.is_empty() {
                let response: StratumPacket<PP, SP> = serde_json::from_str(&buf)?;
                return Ok(response);
            };
        }
    }

    async fn send<T: Serialize>(&self, message: T) -> Result<()> {
        let msg = serde_json::to_vec(&message)?;

        let mut stream = self.write_half.lock().await;

        stream.write_all(&msg).await?;

        stream.write_all(b"\n").await?;

        Ok(())
    }

    //Helper function to make sending requests more simple.
    async fn send_request(&self, method: StratumMethod, params: ClientParam<PP, SP>) -> Result<()> {
        let msg = Request {
            //@todo
            id: ID::Str("".to_owned()),
            method,
            params,
        };

        Ok(self.send(msg).await?)
    }

    // async fn send_response(&self, _method: StratumMethod, _result: PoolParams<SP>) -> Result<()> {
    //     Ok(())
    // }

    pub async fn authorize(&self) -> Result<bool> {
        let authorize = self.config.credentials.clone();

        self.send_request(StratumMethod::Authorize, ClientParam::Authorize(authorize))
            .await?;

        let msg = self.next_message().await?;
        let mut authorize_success = false;

        if let StratumPacket::Response(res) = msg {
            if let Some(PoolParam::AuthorizeResult(auth)) = &res.result {
                self.handle_authorize_return(auth).await?;
                authorize_success = true;
            }
        }

        Ok(authorize_success)
    }

    pub async fn subscribe(&self) -> Result<()> {
        let subscribe = self.subscriber_info.lock().await;

        self.send_request(
            StratumMethod::Subscribe,
            ClientParam::Subscribe(subscribe.clone()),
        )
        .await?;

        Ok(())
    }

    //TODO this needs to accept some params, and also return a value I believe.
    //@todo rename to result
    pub async fn handle_authorize_return(&self, _auth: &PP::AuthorizeResult) -> Result<()> {
        Ok(())
    }

    //Public Submit function
    pub async fn submit(&self, share: SP::Submit) -> Result<()> {
        let msg = Message::Submit(share);

        //@todo remove the unwrap here, just need to impl the error
        self.message_tx.lock().await.send(msg).await?;

        Ok(())
    }

    //This function handles the internal message of submit. This allows for a miner to submit
    //a share, and not have the function block. We then send that share in the event loop above.
    async fn handle_submit_message(&self, share: SP::Submit) -> Result<()> {
        *self.new_work.lock().await = false;

        Ok(self
            .send_request(StratumMethod::Submit, ClientParam::Submit(share))
            .await?)
    }

    async fn handle_subscribe_result(&self, info: PP::SubscribeResult) -> Result<()> {
        //@todo can probably remove this now.
        *self.subscribe_id.lock().await = info.id();
        self.subscriber_info.lock().await.set_sid(&info.id());
        Ok(())
    }

    async fn handle_submit_result(&self, _accepted: bool) -> Result<()> {
        Ok(())
    }

    async fn handle_notify(&self, job: SP::Notify) -> Result<()> {
        *self.current_job.lock().await = Some(job);

        Ok(())
    }

    async fn handle_difficulty(&self, diff: f64) -> Result<()> {
        println!("Updating difficulty to {}", diff);
        *self.difficulty.lock().await = diff;

        Ok(())
    }

    //@todo this only works one time. We could probably use the thread_local crate here, but let's
    //wait on it for now. Probably revamping how we run the miner is best instead of reworking this
    //for now.
    pub async fn is_new_work(&self) -> bool {
        *self.new_work.lock().await
    }

    pub async fn get_difficulty(&self) -> f64 {
        *self.difficulty.lock().await
    }

    pub async fn get_work(&self) -> Option<SP::Notify> {
        let work = self.current_job.lock().await;

        // if self.is_new_work().await {
        //     *self.new_work.lock().await = false;
        // }

        work.clone()
    }

    //Pauses the miner until the initial work has been received
    pub async fn wait_for_work(&self) -> Result<()> {
        loop {
            if self.current_job.lock().await.is_some() {
                break;
            }
            async_std::task::sleep(Duration::from_secs(2)).await;
        }

        Ok(())
    }

    pub async fn get_subscriber_id(&self) -> String {
        self.subscribe_id.lock().await.clone()
    }
}
