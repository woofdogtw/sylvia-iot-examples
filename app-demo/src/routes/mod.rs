use std::{
    collections::{HashMap, VecDeque},
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
};

use actix_web::{dev::HttpServiceFactory, error, web};
use sylvia_iot_sdk::{
    mq::{application::ApplicationMgr, Connection, Options as MgrOptions},
    util::err::ErrResp,
};
use url::Url;

use crate::libs::{
    config::{self, Config},
    data_handler::MgrHandler,
    DlData, UlData,
};

mod v1;

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `app-demo`, the APIs are
    /// - `http://host:port/app-demo/api/v1/data/xxx`
    pub scope_path: &'static str,
    pub mq_conns: Arc<Mutex<HashMap<String, Connection>>>,
    pub mgr: Arc<Mutex<ApplicationMgr>>,
    pub latest_uldata: Arc<Mutex<VecDeque<UlData>>>,
    pub latest_dldata: Arc<Mutex<VecDeque<DlData>>>,
}

/// To create resources for the service.
pub async fn new_state(
    scope_path: &'static str,
    conf: &Config,
) -> Result<State, Box<dyn StdError>> {
    let conf = config::apply_default(conf);
    let host_uri = match Url::parse(conf.mq_uri.as_ref().unwrap()) {
        Err(e) => return Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(uri) => uri,
    };

    let mq_conns = Arc::new(Mutex::new(HashMap::new()));
    let latest_uldata = Arc::new(Mutex::new(VecDeque::new()));
    let latest_dldata = Arc::new(Mutex::new(VecDeque::new()));
    let handler = Arc::new(MgrHandler::new(
        latest_uldata.clone(),
        latest_dldata.clone(),
    ));
    let opts = MgrOptions {
        unit_id: conf.unit.as_ref().unwrap().clone(),
        unit_code: conf.unit.as_ref().unwrap().clone(),
        id: conf.code.as_ref().unwrap().clone(),
        name: conf.code.as_ref().unwrap().clone(),
        ..Default::default()
    };
    let mgr = match ApplicationMgr::new(mq_conns.clone(), &host_uri, opts, handler) {
        Err(e) => return Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(mgr) => Arc::new(Mutex::new(mgr)),
    };

    Ok(State {
        scope_path,
        mq_conns,
        mgr,
        latest_uldata,
        latest_dldata,
    })
}

/// To register service URIs in the specified root path.
pub fn new_service(state: &State) -> impl HttpServiceFactory {
    web::scope(state.scope_path)
        .app_data(web::JsonConfig::default().error_handler(|err, _| {
            error::ErrorBadRequest(ErrResp::ErrParam(Some(err.to_string())))
        }))
        .app_data(web::QueryConfig::default().error_handler(|err, _| {
            error::ErrorBadRequest(ErrResp::ErrParam(Some(err.to_string())))
        }))
        .app_data(web::Data::new(state.clone()))
        .service(v1::data::new_service("/api/v1/data"))
}
