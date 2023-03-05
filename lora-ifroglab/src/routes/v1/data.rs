use actix_web::{dev::HttpServiceFactory, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};

use super::super::State;
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

pub fn new_service(scope_path: &str) -> impl HttpServiceFactory {
    web::scope(scope_path)
        .service(web::resource("/uldata").route(web::get().to(get_uldata)))
        .service(web::resource("/dldata").route(web::get().to(get_dldata)))
        .service(web::resource("/queue/{network_addr}").route(web::get().to(get_queue)))
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

/// `GET /{base}/api/v1/data/queue/{network_addr}`
async fn get_queue(param: web::Path<GetQueueParam>, state: web::Data<State>) -> impl Responder {
    let data: Vec<DlData> = {
        let mutex = state.queue_dldata.lock().unwrap();
        match (*mutex).get(param.network_addr.as_str()) {
            None => vec![],
            Some(data) => data.iter().map(|x| x.clone()).collect(),
        }
    };
    HttpResponse::Ok().json(&GetQueueRes { data })
}
