mod common;
use std::time::Instant;
use tokio_test::assert_ok;

#[tokio::test]
async fn test_long_running_sockets() -> anyhow::Result<()> {
    common::init();

    let (addr, server_handle, shutdown) = assert_ok!(common::spawn_full_server().await);

    //Set socket sleep timing to 5000 seconds -> The server should terminate this socket within 20
    //seconds.
    let clients = common::generate_connections(1, addr, 5000).await;

    let now = Instant::now();

    for (i, client) in clients.into_iter().enumerate() {
        let result = assert_ok!(client.await);

        assert_eq!(result, i);
    }

    assert!(now.elapsed() <= std::time::Duration::from_secs(20));

    shutdown.cancel();

    let server_result = assert_ok!(server_handle.await);

    assert_ok!(server_result);

    Ok(())
}
