//use std::time::{SystemTime, UNIX_EPOCH};
// use serde::{Deserialize, Serialize};
// use stratum_client::{ClientConfig, StratumClient};
// use stratum_types::traits::{PoolParams, StratumParams};

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Authorize {}

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Submit {}

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Subscribe {}

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct Notify {}

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct HandshakeParams {}

// #[derive(Clone, Debug, Deserialize, Serialize)]
// pub struct HNSPoolParams {}

// impl PoolParams for HNSPoolParams {
//     type Authorize = Authorize;
//     type Authorized = bool;
// }

// impl StratumParams for HandshakeParams {
//     type Submit = Submit;
//     type Subscribe = Subscribe;
//     type Notify = Notify;
// }

//#[async_std::test]
//async fn test_client_connect() {
//    //Ensure that we have a stratum server hosted
//    //TODO create our own using the server crate
//    //
//    // let config = ClientConfig {
//    //     host: "138.68.61.31:9999".to_owned(),
//    //     credentials: AuthorizeHNS {},
//    // };

//    let config = ClientConfig {
//        host: "0.0.0.0:8080".to_owned(),
//        credentials: Authorize {},
//    };

//    // let config = ClientConfig {

//    // }

//    let client: StratumClient<HNSPoolParams, HandshakeParams> =
//        StratumClient::new(config).await.unwrap();

//    client.init_connection().await.unwrap();

//    let new_client = client.clone();
//    let new_client_2 = client.clone();

//    async_std::task::spawn(async move {
//        new_client.start().await.unwrap();
//    });

//    async_std::task::spawn(async move {
//        new_client_2.submit(Submit {}).await.unwrap();
//    });

//    loop {}

//    // assert!(result.is_ok());
//}

//#[async_std::test]
//fn test_client_login() {
//    let config = StratumClientConfig {
//        hostname: "0.0.0.0:3008".to_owned(),
//    };

//    let result = StratumClient::start(config);

//    assert!(result.is_ok());

//    let mut client = result.unwrap();

//    let response = client.login("admin", "admin", "test1");

//    dbg!(response);
//}

//#[test]
//fn test_client_subscribe() {
//    let config = StratumClientConfig {
//        hostname: "0.0.0.0:3008".to_owned(),
//    };

//    let result = StratumClient::start(config);

//    assert!(result.is_ok());

//    let mut client = result.unwrap();

//    let response = client.subscribe("test client 1");

//    //Check response is ok.
//}

//#[test]
//fn test_multiple_clients() {
//    //Make sure SIDs are different for each of thses. TODO
//    let config = StratumClientConfig {
//        hostname: "0.0.0.0:3008".to_owned(),
//    };

//    let config2 = StratumClientConfig {
//        hostname: "127.0.0.1:3008".to_owned(),
//    };

//    let result = StratumClient::start(config);

//    let result2 = StratumClient::start(config2);

//    assert!(result.is_ok());

//    assert!(result2.is_ok());

//    let mut client = result.unwrap();

//    let mut client2 = result2.unwrap();

//    let job = client.subscribe("test client 1");

//    let job2 = client.subscribe("test client 2");

//    //TODO check these are okay.
//    //TODO check sids here as well.
//}

//#[test]
//fn test_get_transactions() {
//    let config = StratumClientConfig {
//        hostname: "0.0.0.0:3008".to_owned(),
//    };

//    let result = StratumClient::start(config);

//    dbg!(&result);

//    assert!(result.is_ok());

//    let mut client = result.unwrap();

//    let job = client.subscribe("test client 1").unwrap();

//    let params = job.get("params").unwrap();

//    let job_id = params[0].as_str().unwrap();

//    let transactions = client.get_transactions(job_id);

//    dbg!(&transactions);
//}

//#[async_std::test]
//fn test_bad_submission() {
//    let config = StratumClientConfig {
//        hostname: "0.0.0.0:3008".to_owned(),
//    };

//    let (client, tx) = StratumClientBuilder::new("test", "test", "0.0.0.0:3008").build();

//    // let result = StratumClient::start(config).await;

//    dbg!(&result);

//    assert!(result.is_ok());

//    let mut client = result.unwrap();

//    assert!(client.login("admin", "admin", "test1").unwrap());

//    let job = client.subscribe("test client 1").unwrap();

//    dbg!(&client);

//    let params = job.get("params").unwrap();

//    let job_id = params[0].as_str().unwrap();

//    let nonce = "123456";

//    let start = SystemTime::now();
//    let since_the_epoch = start
//        .duration_since(UNIX_EPOCH)
//        .expect("Time went backwards");

//    let blocktime = since_the_epoch.as_millis().to_string();

//    let accepted = client.submit(nonce, &blocktime);
//}
