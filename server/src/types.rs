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
    pub(crate) data: [u128; 90],
}

impl VarDiffBuffer {
    pub fn new() -> VarDiffBuffer {
        VarDiffBuffer {
            pos: 0,
            used: 0,
            data: [0; 90],
        }
    }

    pub(crate) fn append(&mut self, time: u128) {
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

        let mut total: u128 = 0;
        for i in 0..count {
            total += self.data[i];
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
    #[must_use]
    pub fn new(ready: bool) -> Self {
        ReadyIndicator(Arc::new(AtomicBool::new(ready)))
    }

    #[must_use]
    pub fn create_new(&self) -> Self {
        //@todo review this, no idea why we do this.
        Self(Arc::clone(&self.0))
    }

    //@todo figure out if relaxed is ok
    pub fn ready(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    pub fn not_ready(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    #[must_use]
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

#[derive(Clone, Debug)]
pub struct GlobalVars {
    pub server_id: u8,
}

impl GlobalVars {
    pub fn new(server_id: u8) -> Self {
        GlobalVars { server_id }
    }
}

//@todo export this publically
#[derive(Clone, Debug)]
pub struct Difficulties {
    pub(crate) current: u64,
    pub(crate) old: u64,
    pub(crate) next: u64,
}

impl Difficulties {
    pub(crate) fn new(current: u64, old: u64, next: u64) -> Self {
        Difficulties { current, old, next }
    }

    pub fn current(&self) -> u64 {
        self.current
    }

    pub fn old(&self) -> u64 {
        self.old
    }

    pub fn next(&self) -> Option<u64> {
        if self.next == 0 {
            None
        } else {
            Some(self.next)
        }
    }

    pub fn update_next(&mut self, next: u64) {
        self.next = next;
    }

    pub(crate) fn shift(&mut self) -> Option<u64> {
        if self.next == 0 {
            None
        } else {
            self.old = self.current;
            self.current = self.next;
            self.next = 0;
            Some(self.current)
        }
    }

    pub(crate) fn set_and_shift(&mut self, current: u64) {
        self.old = self.current;
        self.current = current;
        self.next = 0;
    }
}
