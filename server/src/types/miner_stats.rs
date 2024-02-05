use crate::types::VarDiffBuffer;

//@todo THESE CAN OVERFLOW
#[derive(Debug, Clone)]
pub struct MinerStats {
    pub(crate) accepted: u64,
    pub(crate) stale: u64,
    pub(crate) rejected: u64,
    pub(crate) last_active: u128,
}

//@todo THESE CAN OVERFLOW
#[derive(Debug)]
pub struct VarDiffStats {
    pub(crate) last_timestamp: u128,
    pub(crate) last_retarget_share: u64,
    pub(crate) last_retarget: u128,
    pub(crate) vardiff_buf: VarDiffBuffer,
}

//@todo THESE CAN OVERFLOW
#[derive(Debug)]
pub struct BanStats {
    pub(crate) last_ban_check_share: u64,
    pub(crate) needs_ban: bool,
}
