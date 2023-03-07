const MAX_DIFF: u64 = 9_223_372_036_854_775_808;

//@todo would be nice to be able to compare this with u64 so that we don't have to do any casting
#[derive(Clone, Debug, Copy)]
pub struct Difficulty(u64);

impl Difficulty {
    pub fn zero() -> Self {
        Difficulty(0)
    }

    pub fn is_zero(self) -> bool {
        self.0 == 0
    }

    pub fn as_u64(self) -> u64 {
        self.0
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
