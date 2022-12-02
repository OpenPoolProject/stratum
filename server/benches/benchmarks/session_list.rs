use criterion::{criterion_group, criterion_main, Criterion};
use stratum_server::SessionList;

// This is a struct that tells Criterion.rs to use the "futures" crate's current-thread executor
// use criterion::async_executor::AsyncExecutor;

#[derive(Clone, Default)]
pub struct ConnectionState {}

// Here we have an async function to benchmark
async fn do_something(_session_list: SessionList<ConnectionState>) {
    // Do something async with the size
    // session_list.add_miner()
}

fn from_elem(c: &mut Criterion) {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let list: SessionList<ConnectionState> = SessionList::default();

    // c.bench_with_input(BenchmarkId::new("Session_list"), &list, |b, &s| {
    //     // Insert a call to `to_async` to convert the bencher to async mode.
    //     // The timing loops are the same as with the normal bencher.
    //     b.to_async(FuturesExecutor).iter(|| do_something(s));
    // });

    c.bench_function("Session_list", move |b| {
        b.to_async(&rt).iter(|| do_something(list.clone()))
    });
    // c.bench_with_input(, &list, |b, &s| {
    //     // Insert a call to `to_async` to convert the bencher to async mode.
    //     // The timing loops are the same as with the normal bencher.
    //     b.to_async(FuturesExecutor).iter(|| do_something(s));
    // });
}

criterion_group! {
name = benches;
config = Criterion::default();
targets = from_elem
}

criterion_main!(benches);
