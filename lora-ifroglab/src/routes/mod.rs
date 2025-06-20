use std::{
    collections::{HashMap, VecDeque},
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
};

use axum::Router;
use sylvia_iot_sdk::mq::{Connection, Options as MgrOptions, network::NetworkMgr};
use url::Url;

mod v1;

use crate::libs::{
    DlData, UlData,
    config::{self, Config},
    data_handler::MgrHandler,
    lora_task::{LoraTask, Options as TaskOptions},
};

/// The resources used by this service.
#[derive(Clone)]
pub struct State {
    /// The scope root path for the service.
    ///
    /// For example `lora-ifroglab`, the APIs are
    /// - `http://host:port/lora-ifroglab/api/v1/data/xxx`
    pub scope_path: &'static str,
    pub mq_conns: Arc<Mutex<HashMap<String, Connection>>>,
    pub mgr: Arc<Mutex<NetworkMgr>>,
    pub latest_uldata: Arc<Mutex<VecDeque<UlData>>>,
    pub latest_dldata: Arc<Mutex<VecDeque<DlData>>>,
    pub queue_dldata: Arc<Mutex<HashMap<String, VecDeque<DlData>>>>,
    pub freq: u32,
    pub power: u8,
    _lora_task: LoraTask, // use private to run in background
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
    let queue_dldata = Arc::new(Mutex::new(HashMap::new()));
    let handler = Arc::new(MgrHandler::new(queue_dldata.clone()));
    let opts = MgrOptions {
        unit_id: conf.unit.as_ref().unwrap().clone(),
        unit_code: conf.unit.as_ref().unwrap().clone(),
        id: conf.code.as_ref().unwrap().clone(),
        name: conf.code.as_ref().unwrap().clone(),
        ..Default::default()
    };
    let mgr = match NetworkMgr::new(mq_conns.clone(), &host_uri, opts, handler) {
        Err(e) => return Err(Box::new(IoError::new(ErrorKind::InvalidInput, e))),
        Ok(mgr) => Arc::new(Mutex::new(mgr)),
    };

    let latest_uldata = Arc::new(Mutex::new(VecDeque::new()));
    let latest_dldata = Arc::new(Mutex::new(VecDeque::new()));
    let opts = TaskOptions {
        mgr: mgr.clone(),
        latest_uldata: latest_uldata.clone(),
        latest_dldata: latest_dldata.clone(),
        queue_dldata: queue_dldata.clone(),
        dev_path: conf.dev_path.unwrap(),
        freq: conf.freq.unwrap(),
        power: conf.power.unwrap(),
    };

    Ok(State {
        scope_path,
        mq_conns,
        mgr,
        latest_uldata,
        latest_dldata,
        queue_dldata,
        freq: conf.freq.unwrap(),
        power: conf.power.unwrap(),
        _lora_task: LoraTask::new(opts)?,
    })
}

/// To register service URIs in the specified root path.
pub fn new_service(state: &State) -> Router {
    Router::new().nest(
        state.scope_path,
        Router::new().merge(v1::data::new_service("/api/v1/data", state)),
    )
}
