use crate::Result;
use axum::extract::State;
use hyper::StatusCode;
use std::net::TcpListener;
use tokio::task::JoinHandle;
// use std::time::Instant;
// @todo think about using ready indicator
// use crate::types::ReadyIndicator;
use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;

//@todo document this in the readme.
//@todo list all currently banned IPs/Miners.
//@todo list all current miners and their difficulty, etc.
//@todo launch a separate API for metrics

#[derive(Clone)]
pub struct ApiContext {}

// @todo let's probably move API to it's own folder so we can spread things out more.

//The <> here is state, so add to it as we need it.
pub async fn init_api_server(
    state: ApiContext,
    listener: TcpListener,
) -> Result<JoinHandle<Result<()>>> {
    //@todo we need to handle graceful shutdown here.
    let app = Router::with_state(state)
        //@todo match these with the others so they are in health.
        .route("/livez", get(livez))
        .route("/readyz", get(readyz))
        .layer(
            //@todo set these more explicitly so that we lock down the sercurity of this bad
            //boy.
            CorsLayer::new()
                .allow_origin("*".parse::<HeaderValue>().unwrap())
                .allow_methods([Method::GET]),
        );

    let server = axum::Server::from_tcp(listener)?.serve(app.into_make_service());

    let handle = tokio::spawn(async move {
        server.await?;

        Ok(())
    });

    Ok(handle)
}

async fn livez() -> StatusCode {
    StatusCode::OK
}

async fn readyz(State(_state): State<ApiContext>) -> StatusCode {
    // if state.ready.status().await {
    //     StatusCode::OK
    // } else {
    //     StatusCode::BAD_REQUEST
    // }
    StatusCode::OK
}

//@todo 2 checks we want here.
//1. Basic healthcheck, literally just an http endoint /ping that returns 200
//2. More complex that checks all dependent services are working. That might be able to be saved by
//   whatever is implementing this above, but we can brainstorm more on this one.
