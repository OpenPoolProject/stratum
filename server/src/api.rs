use crate::Result;
use async_std::task;
// use std::time::Instant;
use crate::types::ReadyIndicator;
use log::{info, warn};
use stop_token::{future::FutureExt, StopToken};
use tide::{Middleware, Next, Request, Response};

//@todo document this in the readme.
//@todo list all currently banned IPs/Miners.
//@todo list all current miners and their difficulty, etc.

#[derive(Clone)]
pub struct State {
    ready_indicator: ReadyIndicator,
}

//The <> here is state, so add to it as we need it.
pub async fn init_api_server(stop_token: StopToken, ready_indicator: ReadyIndicator) -> Result<()> {
    let mut app = tide::with_state(State { ready_indicator });
    app.at("/healthz").get(health_check);
    app.at("/status").get(status);
    app.at("/ready").get(readiness);
    // app.at("/metrics").get(prom_metrics);

    //@todo come from config.
    task::spawn(async move {
        let listen = app.listen("0.0.0.0:8081").timeout_at(stop_token);
        match listen.await {
            Ok(result) => {
                match result {
                    Ok(_) => info!("Api Server has cleanly shutdown"),
                    Err(e) => warn!("Api Server shutting down on error: {}", e),
                };
            }
            Err(_) => info!("API Server shutdown. Received stop token."),
        }
    });
    Ok(())
}

// This is an example of middleware that keeps its own state and could
// be provided as a third party crate
#[derive(Default)]
struct MetricsMiddleware {}

#[tide::utils::async_trait]
impl<State: Clone + Send + Sync + 'static> Middleware<State> for MetricsMiddleware {
    async fn handle(&self, req: Request<State>, next: Next<'_, State>) -> tide::Result {
        // let start = Instant::now();
        let res = next.run(req).await;

        Ok(res)
    }
}

//@todo number of workers
async fn status(_req: tide::Request<State>) -> tide::Result {
    Ok(Response::new(200))
}

async fn health_check(_req: tide::Request<State>) -> tide::Result {
    Ok(Response::new(200))
}

async fn readiness(req: tide::Request<State>) -> tide::Result {
    let ready = req.state().ready_indicator.inner().await;

    if ready {
        Ok(Response::new(200))
    } else {
        Ok(Response::new(500))
    }
}

// pub async fn prom_metrics<CState: Clone + Send + Sync + 'static>(
//     req: Request<State<CState>>,
// ) -> tide::Result {

//     metrics::recorder().
//     let encoder = TextEncoder::new();
//     let metric_families = prometheus::gather();
//     // Encode them to send.
//     let mut buffer = vec![];

//     encoder.encode(&metric_families, &mut buffer).unwrap();

//     let response = Response::builder(200)
//         .header("content-type", encoder.format_type())
//         .body(Body::from(buffer))
//         .build();

//     Ok(response)
// }

//@todo 2 checks we want here.
//1. Basic healthcheck, literally just an http endoint /ping that returns 200
//2. More complex that checks all dependent services are working. That might be able to be saved by
//   whatever is implementing this above, but we can brainstorm more on this one.
