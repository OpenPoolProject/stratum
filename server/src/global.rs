use crate::ConnectionList;
use async_std::{future::Future, sync::Arc};
use async_trait::async_trait;

#[async_trait]
pub trait Global<State: Clone + Send + Sync + 'static, CState: Clone + Send + Sync + 'static>:
    Send + Sync + 'static
{
    async fn call(&self, state: State, connection_list: Arc<ConnectionList<CState>>);
}

#[async_trait]
impl<State, CState, F, Fut> Global<State, CState> for F
where
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(State, Arc<ConnectionList<CState>>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    async fn call(&self, state: State, connection_list: Arc<ConnectionList<CState>>) {
        let fut = (self)(state.clone(), connection_list.clone());

        fut.await;
    }
}
