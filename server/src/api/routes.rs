use crate::{api::ApiContext, ban_manager::BanInfo};
use axum::{extract::State, Json};
use hyper::StatusCode;

pub(crate) async fn livez() -> StatusCode {
    StatusCode::OK
}

pub(crate) async fn readyz(State(state): State<ApiContext>) -> StatusCode {
    if state.ready_indicator.status() {
        StatusCode::OK
    } else {
        StatusCode::BAD_REQUEST
    }
}

pub(crate) async fn get_banned(State(state): State<ApiContext>) -> Json<Vec<BanInfo>> {
    Json(state.ban_manager.temp_bans())
}
