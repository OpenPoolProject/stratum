use crate::{
    route::{DynEndpoint, Endpoint},
    types::GlobalVars,
    Frame, Session, StratumRequest,
};
use std::collections::HashMap;
use tracing::warn;

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
        value: Frame,
        state: State,
        connection: Session<CState>,
        global_vars: GlobalVars,
    ) {
        let Some(endpoint) = self.routes.get(value.method()) else {
                warn!("Method {} was not found", value.method());
                return;
        };

        // if log::log_enabled!(log::Level::Trace) {
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
        tracing::trace!(
            "Calling method: {} for connection: {}",
            value.method(),
            connection.id()
        );
        // }
        // }

        let request = StratumRequest {
            state,
            values: value,
            global_vars,
        };

        endpoint.call(request, connection).await;
    }
}
