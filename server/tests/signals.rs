mod common;

use common::spawn_full_server;
use tokio_test::assert_ok;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

//===== SIGINT Tests =====//
#[tokio::test]
async fn test_signal_sigint_clean_shutdown() {
    common::init();

    let (_, server_handle) = assert_ok!(spawn_full_server().await);

    common::call_sigint();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);
}

#[tokio::test]
async fn test_signal_sigint_clean_shutdown_with_connection() {
    common::init();

    let (addr, server_handle) = assert_ok!(spawn_full_server().await);

    let clients = common::generate_connections(1, addr, 5).await;

    common::call_sigint();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_signal_sigint_clean_shutdown_with_n_connections() {
    common::init();

    let (addr, server_handle) = assert_ok!(spawn_full_server().await);

    let clients = common::generate_connections(10, addr, 5).await;

    common::call_sigint();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

//===== SIGTERM Tests =====//
#[tokio::test]
async fn test_sigterm_clean_shutdown() {
    common::init();

    let (_, server_handle) = assert_ok!(spawn_full_server().await);

    common::call_sigterm();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);
}

#[tokio::test]
async fn test_signal_sigterm_clean_shutdown_with_connection() {
    common::init();

    let (addr, server_handle) = assert_ok!(spawn_full_server().await);

    let clients = common::generate_connections(1, addr, 5).await;

    common::call_sigterm();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_signal_sigterm_clean_shutdown_with_n_connections() {
    common::init();

    let (addr, server_handle) = assert_ok!(spawn_full_server().await);

    let clients = common::generate_connections(10, addr, 5).await;

    common::call_sigterm();

    let server_result = assert_ok!(server_handle.await);

    //@todo In the future, make sure the Result is Err(Shutdown::Signal(Correct Signal)) Something like this.

    assert_ok!(server_result);

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}
