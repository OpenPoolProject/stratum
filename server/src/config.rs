#[derive(Clone, Debug, Default)]
pub struct VarDiffConfig {
    pub(crate) var_diff: bool,
    pub(crate) minimum_difficulty: u64,
    pub(crate) maximum_difficulty: u64,
    //Seconds
    pub(crate) retarget_time: u64,
    //Seconds
    pub(crate) target_time: u64,
    pub(crate) variance_percent: f64,
}

#[derive(Clone, Debug, Default)]
pub struct UpstreamConfig {
    pub(crate) enabled: bool,
    pub(crate) url: String,
}
