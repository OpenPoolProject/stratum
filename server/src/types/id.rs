use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ID {
    Num(u64),
    Str(String),
    Null(serde_json::Value),
}

impl ID {
    #[must_use]
    pub fn null() -> ID {
        ID::Null(serde_json::Value::Null)
    }
}

impl std::fmt::Display for ID {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ID::Num(ref e) => write!(f, "{e}"),
            ID::Str(ref e) => write!(f, "{e}"),
            ID::Null(ref _e) => write!(f, "null"),
        }
    }
}
