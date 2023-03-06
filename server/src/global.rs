use crate::SessionList;
use async_trait::async_trait;
use futures::Future;

#[async_trait]
pub trait Global<State: Clone, CState: Clone>: Send + Sync + 'static {
    async fn call(&self, state: State, session_list: SessionList<CState>);
}

#[async_trait]
impl<State, CState, F, Fut> Global<State, CState> for F
where
    State: Clone + Send + Sync + 'static,
    CState: Clone + Send + Sync + 'static,
    F: Send + Sync + 'static + Fn(State, SessionList<CState>) -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    async fn call(&self, state: State, session_list: SessionList<CState>) {
        let fut = (self)(state.clone(), session_list.clone());

        fut.await;
    }
}
