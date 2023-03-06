mod difficulties;
mod difficulty;
mod difficulty_settings;
mod id;
mod ready_indicator;
mod session_id;
mod var_diff_buffer;

pub use difficulties::Difficulties;
pub use difficulty::Difficulty;
pub use difficulty_settings::DifficultySettings;
pub use id::ID;
pub use ready_indicator::ReadyIndicator;
pub use session_id::SessionID;
pub use var_diff_buffer::VarDiffBuffer;

pub const EX_MAGIC_NUMBER: u8 = 0x7F;

#[derive(Clone, Debug)]
pub struct GlobalVars {
    pub server_id: u8,
}

impl GlobalVars {
    pub fn new(server_id: u8) -> Self {
        GlobalVars { server_id }
    }
}
