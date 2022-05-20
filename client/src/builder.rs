// use crate::types::{Job, Share, Target};
// use crate::StratumClient;
// use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
// use futures::lock::Mutex;
// use std::sync::Arc;

// pub struct StratumClientBuilder {
//     hostname: String,
//     username: String,
//     password: String,
//     job: Job,
//     extra_nonce: u64,
//     target: Target,
// }

// impl StratumClientBuilder {
//     fn new(username: &str, password: &str, hostname: &str) -> Self {
//         StratumClientBuilder {
//             username: username.to_owned(),
//             password: password.to_owned(),
//             hostname: hostname.to_owned(),
//             job: Job::default(),
//             extra_nonce: 0,
//             target: Target::default(),
//         }
//     }

//     fn build(self) -> (StratumClient, UnboundedSender<Share>) {
//         let (tx, rx): (UnboundedSender<Share>, UnboundedReceiver<Share>) = unbounded();

//         (
//             StratumClient {
//                 hostname: self.hostname,
//                 username: self.username,
//                 password: self.password,
//                 authorized: Arc::new(Mutex::new(false)),
//                 current_job: Arc::new(Mutex::new(self.job)),
//                 extra_nonce: Arc::new(Mutex::new(self.extra_nonce)),
//                 target: Arc::new(Mutex::new(self.target)),
//                 writer: Arc::new(Mutex::new(None)),
//                 rx: Arc::new(Mutex::new(rx)),
//             },
//             tx,
//         )
//     }
// }
