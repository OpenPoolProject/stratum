use crate::{api::Context, Result};
use std::net::{SocketAddr, TcpListener};
use tokio::task::JoinHandle;
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
    pub fn build(addr: SocketAddr, state: Context) -> Result<Self> {
        let tcp = TcpListener::bind(addr)?;
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
            let app = Router::with_state(self.state.clone())
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
                );

            let server = axum::Server::from_tcp(listener)?
                .serve(app.into_make_service())
                .with_graceful_shutdown(async move { cancel_token.cancelled().await });
            let handle = tokio::spawn(async move {
                server.await?;

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
