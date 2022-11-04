use crate::{Error, Result};
use extended_primitives::Buffer;
use serde::{Deserialize, Serialize};
use std::{
    fmt,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};

#[derive(Debug)]
pub struct VarDiffBuffer {
    pub(crate) pos: usize,
    pub(crate) used: usize,
    pub(crate) data: [i64; 90],
}

impl VarDiffBuffer {
    pub fn new() -> VarDiffBuffer {
        VarDiffBuffer {
            pos: 0,
            used: 0,
            data: [0; 90],
        }
    }

    pub(crate) fn append(&mut self, time: i64) {
        self.data[self.pos] = time;
        self.pos += 1;
        self.pos %= 90;

        if self.used < 90 {
            self.used += 1;
        }
    }

    pub(crate) fn reset(&mut self) {
        self.pos = 0;
        self.used = 0;
    }

    pub(crate) fn avg(&self) -> f64 {
        let mut count = 90;

        if self.used < 90 {
            count = self.pos;
        }

        let mut total: i64 = 0;
        for i in 0..count {
            total += self.data[i]
        }

        (total as f64) / (count as f64)
    }
}

pub const EX_MAGIC_NUMBER: u8 = 0x7F;

// #[derive(Clone, Default)]
// pub struct ReadyIndicator(Arc<Mutex<bool>>);

//@todo testing this
#[derive(Default, Clone)]
pub struct ReadyIndicator(Arc<AtomicBool>);

impl ReadyIndicator {
    pub fn new(ready: bool) -> Self {
        ReadyIndicator(Arc::new(AtomicBool::new(ready)))
    }

    //@todo review this, no idea why we do this.
    pub fn create_new(&self) -> Self {
        Self(Arc::clone(&self.0))
    }

    //@todo figure out if relaxed is ok
    pub fn ready(&self) {
        self.0.store(true, Ordering::Relaxed)
    }

    pub fn not_ready(&self) {
        self.0.store(false, Ordering::Relaxed)
    }

    pub fn status(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(untagged)]
pub enum ID {
    Num(u64),
    Str(String),
    Null(serde_json::Value),
}

impl ID {
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

#[derive(Clone, Debug)]
pub struct GlobalVars {
    pub server_id: u8,
}

impl GlobalVars {
    pub fn new(server_id: u8) -> Self {
        GlobalVars { server_id }
    }
}
