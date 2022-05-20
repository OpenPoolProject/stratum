use crate::{
    route::{DynEndpoint, Endpoint},
    types::{GlobalVars, MessageValue},
    Connection, StratumRequest,
};
use async_std::sync::Arc;
use log::warn;
use std::collections::HashMap;

pub struct Router<State, CState> {
    routes: HashMap<String, Box<DynEndpoint<State, CState>>>,
}

impl<State: Clone + Send + Sync + 'static, CState: Clone + Send + Sync + 'static>
    Router<State, CState>
{
    pub fn new() -> Router<State, CState> {
        Router {
            routes: HashMap::new(),
        }
    }

    pub fn add(&mut self, method: &str, ep: impl Endpoint<State, CState>) {
        self.routes.insert(method.to_owned(), Box::new(ep));
    }

    pub async fn call(
        &self,
        method: &str,
        value: MessageValue,
        state: State,
        connection: Arc<Connection<CState>>,
        global_vars: GlobalVars,
    ) {
        let endpoint = match self.routes.get(method) {
            Some(endpoint) => endpoint,
            None => {
                warn!("Method {} was not found", method);
                return;
            }
        };

        if log::log_enabled!(log::Level::Trace) {
            //@todo I think there is something really good we have going here, but needs to be
            //refined a bit more before it's ready.
            //@todo what we need to do here is idenitfy any message that is session specific -> I
            //think one of the newer builds of btccom pool does this.
            // if let MessageValue::ExMessage(msg) = value {
            // if connection.is_agent().await {
            // log::trace!(
            //     "Calling method: {} for connection: {} with miner ID: {}",
            //     method,
            //     connection.id(),
            // );
            // } else {
            log::trace!(
                "Calling method: {} for connection: {}",
                method,
                connection.id()
            );
            // }
        }

        let request = StratumRequest {
            state,
            values: value,
            global_vars,
        };

        endpoint.call(request, connection).await;
    }
}
