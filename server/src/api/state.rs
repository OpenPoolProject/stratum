use crate::{ban_manager, ReadyIndicator};

#[derive(Clone)]
pub struct Context {
    pub(crate) ban_manager: ban_manager::Handle,
    pub(crate) ready_indicator: ReadyIndicator,
}
