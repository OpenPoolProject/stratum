#![no_main]
use libfuzzer_sys::fuzz_target;

//@todo so what we'll want do to here is have the fuzz data go through one end of the stream such
//that our own server is listening to it on the other side. SOo basically we will spin up the whole
//server w/ moxck data and then connect a stream to the pool, and then start fuzzing data through
//that.

fuzz_target!(|data: &[u8]| {
    // fuzzed code goes here
});
