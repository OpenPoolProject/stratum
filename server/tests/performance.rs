use rlimit::{setrlimit, Resource};

const DEFAULT_SOFT_LIMIT: u64 = 4 * 1024 * 1024;
const DEFAULT_HARD_LIMIT: u64 = 8 * 1024 * 1024;

//@todo https://docs.rs/rlimit/latest/rlimit/
pub fn fix_ulimit() {
    assert!(Resource::FSIZE
        .set(DEFAULT_SOFT_LIMIT, DEFAULT_HARD_LIMIT)
        .is_ok());

    let soft = 16384;
    let hard = soft * 2;
    assert!(setrlimit(Resource::NOFILE, soft, hard).is_ok());
}
