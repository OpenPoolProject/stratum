use crate::api::{Context, Result};
use std::{future::IntoFuture, net::SocketAddr};
use tokio::{net::TcpListener, task::JoinHandle};
use tokio_util::sync::CancellationToken;
// use std::time::Instant;
// @todo think about using ready indicator
// use crate::types::ReadyIndicator;
use super::routes;
use axum::{
    http::{HeaderValue, Method},
    routing::get,
    Router,
};
use tower_http::cors::CorsLayer;

pub struct Api {
    pub(crate) info: SocketAddr,
    pub(crate) listener: Option<TcpListener>,
    pub(crate) state: Context,
    // pub(crate) listener:
    //     Option<Server<AddrIncoming, axum::routing::IntoMakeService<axum::Router<ApiContext>>>>,
    // pub(crate) listener: Option<TcpListener>,
}

impl Api {
    pub async fn build(addr: SocketAddr, state: Context) -> Result<Self> {
        let tcp = TcpListener::bind(addr).await?;
        let info = tcp.local_addr()?;

        Ok(Self {
            info,
            listener: Some(tcp),
            state,
        })
    }

    //@todo we can also wait to receive state until this function, then we don't need to store it,
    //and we get access to everything later..
    //Runs the Api server in the background
    pub fn run(&mut self, cancel_token: CancellationToken) -> Result<JoinHandle<Result<()>>> {
        // self.listener.await?;

        let listener = self.listener.take();
        if let Some(listener) = listener {
            let app = Router::new()
                //@todo match these with the others so they are in health.
                .route("/livez", get(routes::livez))
                .route("/readyz", get(routes::readyz))
                //@todo but we do probs want an "add banned"
                .route(
                    "/banned",
                    get(routes::get_banned).post(routes::remove_banned),
                )
                .layer(
                    //@todo set these more explicitly so that we lock down the sercurity of this bad
                    //boy.
                    CorsLayer::new()
                        .allow_origin("*".parse::<HeaderValue>().unwrap())
                        .allow_methods([Method::GET]),
                )
                .with_state(self.state.clone());

            //@todo quite annoying, but the new axum version essentially makes us do a massive redo
            //for graceful shutdown.... So we will need to wait some time to revamp this, for now
            //graceful shutdown DOES NOT work.
            //Actually now I'm looking at a different example, and it seems straight forward....
            //Not sure what to think here, but going to use the new example, and come back to this
            //later.
            //@todo looks like AXUM will be adding back in graceful shutdown on axum::serve, so
            //let's update this when that is released
            // let server = axum::Server::from_tcp(listener)?
            //     .serve(app.into_make_service())
            //     .with_graceful_shutdown(async move { cancel_token.cancelled().await });
            let handle = tokio::spawn(async move {
                //@todo not quite sure why we use into_future here rather than just leaving it but
                //let's see
                let server = axum::serve(listener, app.into_make_service()).into_future();

                tokio::select! {
                    biased;

                    () = cancel_token.cancelled() => {},

                    result = server => {
                        result?;
                },

                }

                Ok(())
            });

            Ok(handle)
        } else {
            //@todo make an apiError
            // Err(Error::)
            todo!()
        }

        // Ok(())
    }

    pub(crate) fn listen_address(&self) -> SocketAddr {
        self.info
    }
}
