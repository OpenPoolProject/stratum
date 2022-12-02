use std::time::{SystemTime, UNIX_EPOCH};

const MAX_DIFF: u64 = 9_223_372_036_854_775_808;

#[must_use]
pub fn format_difficulty(diff: u64) -> u64 {
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

pub fn now() -> u128 {
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");

    since_the_epoch.as_millis()
}
