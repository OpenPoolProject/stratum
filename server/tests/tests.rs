//@todo test to ensure startup time is under some limit (Set that as a const in tests thne.)
//@todo see Vector tests and tikv and linkered.
//
////@todo tests for various allocators. as well as some benchmarks
mod common;

use std::time::Duration;

use common::spawn_full_server;
use tokio_test::assert_ok;

#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

#[tokio::test]
async fn test_heap_allocation() -> anyhow::Result<()> {
    let _profiler = dhat::Profiler::builder().testing().build();

    common::init();

    let (addr, server_handle, shutdown) = assert_ok!(spawn_full_server().await);

    let stats = dhat::HeapStats::get();

    dbg!(stats);

    // Generate Clients
    let clients = common::generate_connections(1, addr, 5).await;

    // Check post client stats
    let stats = dhat::HeapStats::get();

    dbg!(stats);

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }

    //@todo I think if I move this to the front of the function, the allocations will stay the same
    //so then I can compare them from here and above.
    tokio::time::sleep(Duration::from_secs(1)).await;

    // Check post client stats
    let stats = dhat::HeapStats::get();

    dbg!(stats);

    tokio::time::sleep(Duration::from_secs(2)).await;

    shutdown.cancel();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);

    Ok(())
}
