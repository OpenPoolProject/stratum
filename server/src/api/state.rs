use crate::{BanManagerHandle, ReadyIndicator};

#[derive(Clone)]
pub struct ApiContext {
    pub(crate) ban_manager: BanManagerHandle,
    pub(crate) ready_indicator: ReadyIndicator,
}
