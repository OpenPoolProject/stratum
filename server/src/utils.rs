const MAX_DIFF: u64 = 9223372036854775808;

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
