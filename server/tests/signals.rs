use std::time::Duration;
use tokio_test::assert_ok;

pub mod common;

#[cfg(not(target_env = "msvc"))]
use jemallocator::Jemalloc;

#[cfg(not(target_env = "msvc"))]
#[global_allocator]
static GLOBAL: Jemalloc = Jemalloc;

//===== SIGINT Tests =====//
#[tokio::test]
async fn test_signal_sigint_clean_shutdown() {
    common::init();

    let port = common::find_port().await;

    let server = tokio::spawn(async move {
        let mut server = common::server_with_auth(port).await;
        server.start().await
    });

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigint();

    let result = server.await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_signal_sigint_clean_shutdown_with_connection() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_auth(port).await;
        server.start().await
    });

    let clients = common::generate_connections(1, &format!("0.0.0.0:{port}"), 5);

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigint();

    let result = server.await;

    assert!(result.is_ok());

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_signal_sigint_clean_shutdown_with_n_connections() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_auth(port).await;
        server.start().await
    });

    let clients = common::generate_connections(10, &format!("0.0.0.0:{port}"), 5);

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigint();

    let result = server.await;

    assert!(result.is_ok());

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_signal_sigint_with_infinite_global() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_global(port).await;
        server.start().await
    });

    //@todo maybe just put this into the call_sigint function and await on it.
    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigint();

    let result = server.await;

    assert!(result.is_ok());
}

//===== SIGTERM Tests =====//
#[tokio::test]
async fn test_sigterm_clean_shutdown() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_auth(port).await;
        server.start().await
    });

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigterm();

    let result = server.await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn test_signal_sigterm_clean_shutdown_with_connection() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_auth(port).await;
        server.start().await
    });

    let clients = common::generate_connections(1, &format!("0.0.0.0:{port}"), 5);

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigterm();

    let result = server.await;

    assert!(result.is_ok());

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_signal_sigterm_clean_shutdown_with_n_connections() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_auth(port).await;
        server.start().await
    });

    let clients = common::generate_connections(10, &format!("0.0.0.0:{port}"), 5);

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigterm();

    let result = server.await;

    assert!(result.is_ok());

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }
}

#[tokio::test]
async fn test_signal_sigterm_with_infinite_global() {
    common::init();

    let port = common::find_port().await;
    let server = tokio::spawn(async move {
        let mut server = common::server_with_global(port).await;
        server.start().await
    });

    // let clients = common::generate_connections(10, &format!("0.0.0.0:{}", port), 5);

    //Give the server time to register the hooks.
    tokio::time::sleep(Duration::from_secs(2)).await;

    common::call_sigterm();

    let result = server.await;

    assert!(result.is_ok());
}
