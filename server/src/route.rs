use crate::{Connection, StratumRequest};
use async_std::{future::Future, sync::Arc};
use async_trait::async_trait;

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
            Err(_e) => {
                connection.disconnect().await;
                serde_json::Value::Null
            }
        }
    }
}
