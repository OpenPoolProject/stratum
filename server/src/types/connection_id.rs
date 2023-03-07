use std::fmt::Display;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ConnectionID(Uuid);

impl ConnectionID {
    pub fn new() -> Self {
        ConnectionID(Uuid::new_v4())
    }
}

impl Default for ConnectionID {
    fn default() -> Self {
        ConnectionID(Uuid::new_v4())
    }
}

impl Display for ConnectionID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
