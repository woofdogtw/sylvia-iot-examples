use actix_web::{dev::HttpServiceFactory, web, HttpResponse, Responder};
use chrono::Utc;
use log::error;
use serde::{Deserialize, Serialize};
use sylvia_iot_sdk::{
    mq::application::DlData as AppDlData,
    util::{err::ErrResp, strings},
};

use super::super::State;
use crate::libs::{DlData, UlData, MAX_DATA};

#[derive(Deserialize)]
struct PostDlDataReq {
    data: PostDlData,
}

#[derive(Deserialize)]
struct PostDlData {
    #[serde(rename = "networkCode")]
    network_code: String,
    #[serde(rename = "networkAddr")]
    network_addr: String,
    payload: String,
}

#[derive(Serialize)]
struct GetUlDataRes {
    data: Vec<UlData>,
}

#[derive(Serialize)]
struct GetDlDataRes {
    data: Vec<DlData>,
}

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("/uldata").route(web::get().to(get_uldata)))
        .service(
            web::resource("/dldata")
                .route(web::get().to(get_dldata))
                .route(web::post().to(post_dldata)),
        )
}

/// `GET /{base}/api/v1/data/uldata`
async fn get_uldata(state: web::Data<State>) -> impl Responder {
    let data: Vec<UlData> = {
        let mutex = state.latest_uldata.lock().unwrap();
        (*mutex).iter().map(|x| x.clone()).collect()
    };
    HttpResponse::Ok().json(&GetUlDataRes { data })
}

/// `GET /{base}/api/v1/data/dldata`
async fn get_dldata(state: web::Data<State>) -> impl Responder {
    let data: Vec<DlData> = {
        let mutex = state.latest_dldata.lock().unwrap();
        (*mutex).iter().map(|x| x.clone()).collect()
    };
    HttpResponse::Ok().json(&GetDlDataRes { data })
}

/// `POST /{base}/api/v1/data/dldata`
async fn post_dldata(body: web::Json<PostDlDataReq>, state: web::Data<State>) -> impl Responder {
    const FN_NAME: &'static str = "post_dldata";
    let now = Utc::now();
    let data = DlData {
        correlation_id: strings::random_id(&now, 4),
        data_id: None,
        time: strings::time_str(&now),
        network_code: body.data.network_code.clone(),
        network_addr: body.data.network_addr.clone(),
        data: body.data.payload.clone(),
        status: -2,
        error: None,
        message: None,
    };

    let msg = AppDlData {
        correlation_id: data.correlation_id.clone(),
        device_id: None,
        network_code: Some(data.network_code.clone()),
        network_addr: Some(data.network_addr.clone()),
        data: data.data.clone(),
        extension: None,
    };

    {
        let mut mutex = state.latest_dldata.lock().unwrap();
        (*mutex).push_back(data);
        if (*mutex).len() > MAX_DATA {
            (*mutex).pop_front();
        }
    }

    {
        if let Err(e) = state.mgr.lock().unwrap().send_dldata(&msg) {
            error!("[{}] send broker payload error: {}", FN_NAME, e);
            return Err(ErrResp::ErrIntMsg(Some(format!(
                "send broker payload error: {}",
                e
            ))));
        }
    }
    Ok(HttpResponse::NoContent().finish())
}
