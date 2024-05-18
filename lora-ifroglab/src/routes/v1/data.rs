use axum::{extract::State, response::IntoResponse, routing, Router};
use serde::{Deserialize, Serialize};
use sylvia_iot_sdk::util::http::{Json, Path};

use super::super::State as AppState;
use crate::libs::{DlData, UlData};

#[derive(Deserialize)]
struct GetQueueParam {
    network_addr: String,
}

#[derive(Serialize)]
struct GetUlDataRes {
    data: Vec<UlData>,
}

#[derive(Serialize)]
struct GetDlDataRes {
    data: Vec<DlData>,
}

#[derive(Serialize)]
struct GetQueueRes {
    data: Vec<DlData>,
}

pub fn new_service(scope_path: &str, state: &AppState) -> Router {
    Router::new().nest(
        scope_path,
        Router::new()
            .route("/uldata", routing::get(get_uldata))
            .route("/dldata", routing::get(get_dldata))
            .route("/queue/:network_addr", routing::get(get_queue))
            .with_state(state.clone()),
    )
}

/// `GET /{base}/api/v1/data/uldata`
async fn get_uldata(State(state): State<AppState>) -> impl IntoResponse {
    let data: Vec<UlData> = {
        let mutex = state.latest_uldata.lock().unwrap();
        (*mutex).iter().map(|x| x.clone()).collect()
    };
    Json(GetUlDataRes { data })
}

/// `GET /{base}/api/v1/data/dldata`
async fn get_dldata(State(state): State<AppState>) -> impl IntoResponse {
    let data: Vec<DlData> = {
        let mutex = state.latest_dldata.lock().unwrap();
        (*mutex).iter().map(|x| x.clone()).collect()
    };
    Json(GetDlDataRes { data })
}

/// `GET /{base}/api/v1/data/queue/{network_addr}`
async fn get_queue(
    State(state): State<AppState>,
    Path(param): Path<GetQueueParam>,
) -> impl IntoResponse {
    let data: Vec<DlData> = {
        let mutex = state.queue_dldata.lock().unwrap();
        match (*mutex).get(param.network_addr.as_str()) {
            None => vec![],
            Some(data) => data.iter().map(|x| x.clone()).collect(),
        }
    };
    Json(GetQueueRes { data })
}
