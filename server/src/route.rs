use crate::{Connection, StratumRequest};
use async_trait::async_trait;
use futures::Future;
use std::sync::Arc;
use tracing::error;

pub(crate) type DynEndpoint<State, CState> = dyn Endpoint<State, CState>;

#[async_trait]
pub trait Endpoint<State: Clone + Send + Sync + 'static, CState: Clone + Send + Sync + 'static>:
    Send + Sync + 'static
{
    async fn call(
        &self,
        req: StratumRequest<State>,
        connection: Arc<Connection<CState>>,
    ) -> serde_json::Value;
}

#[async_trait]
impl<State, CState, F, Fut, Res, E> Endpoint<State, CState> for F
where
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(StratumRequest<State>, Arc<Connection<CState>>) -> Fut,
    Fut: Future<Output = std::result::Result<Res, E>> + Send + 'static,
    E: std::error::Error + 'static + std::marker::Send,
    Res: Into<serde_json::Value> + 'static + std::marker::Send,
{
    async fn call(
        &self,
        req: StratumRequest<State>,
        connection: Arc<Connection<CState>>,
    ) -> serde_json::Value {
        let fut = (self)(req, connection.clone());

        match fut.await {
            Ok(res) => res.into(),
            Err(e) => {
                error!(
                    connection_id = connection.id().to_string(),
                    error.cause_chain = ?e,
                    error.message = %e,
                    "Request failed disconnecting miner"
                );

                //@todo better response values here if we can.
                connection.disconnect().await;
                serde_json::Value::Null
            }
        }
    }
}
