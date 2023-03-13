#![allow(clippy::cast_sign_loss)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation)]

use std::ops::Deref;

const MAX_DIFF: u64 = 9_223_372_036_854_775_808;

#[derive(Clone, Debug, Copy)]
pub struct Difficulty(u64);

impl Difficulty {
    #[must_use]
    pub fn zero() -> Self {
        Difficulty(0)
    }

    #[must_use]
    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    #[must_use]
    pub fn as_u64(self) -> u64 {
        self.0
    }

    #[must_use]
    pub fn log2(&self) -> u8 {
        //@todo can we do self.0.ilog(2)
        (self.0 as f64).log2() as u8
    }
}

impl Deref for Difficulty {
    type Target = u64;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<u64> for Difficulty {
    fn from(value: u64) -> Self {
        Difficulty(format_difficulty(value))
    }
}

#[must_use]
fn format_difficulty(diff: u64) -> u64 {
    if diff >= MAX_DIFF {
        return MAX_DIFF;
    }

    let mut new_diff: u64 = 1;
    let mut i = 0;
    while new_diff < diff {
        new_diff <<= 1;
        i += 1;
    }
    assert!(i <= 63);
    1_u64 << i
}
