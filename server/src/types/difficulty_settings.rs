use crate::types::Difficulty;

#[derive(Debug, Clone)]
pub struct DifficultySettings {
    pub(crate) default: Difficulty,
    pub(crate) minimum: Difficulty,
}
