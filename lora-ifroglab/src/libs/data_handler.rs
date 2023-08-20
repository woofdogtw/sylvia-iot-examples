//! Implements data handlers for network data from the broker.

use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use chrono::Utc;
use log::{error, info};
use sylvia_iot_sdk::{
    mq::{
        network::{
            DlData as NetDlData, DlDataResult as NetDlDataResult, EventHandler, NetworkCtrlMsg,
            NetworkMgr,
        },
        MgrStatus,
    },
    util::strings,
};

use super::DlData;

pub struct MgrHandler {
    queue_dldata: Arc<Mutex<HashMap<String, VecDeque<DlData>>>>,
}

impl MgrHandler {
    pub fn new(queue_dldata: Arc<Mutex<HashMap<String, VecDeque<DlData>>>>) -> Self {
        MgrHandler { queue_dldata }
    }
}

#[async_trait]
impl EventHandler for MgrHandler {
    async fn on_status_change(&self, _mgr: &NetworkMgr, _status: MgrStatus) {}

    async fn on_dldata(&self, mgr: &NetworkMgr, data: Box<NetDlData>) -> Result<(), ()> {
        const FN_NAME: &'static str = "MgrHandler::on_dldata";

        let addr = &data.network_addr;

        let push_data = DlData {
            data_id: data.data_id.clone(),
            time: strings::time_str(&Utc::now()),
            publish: strings::time_str(&data.publish),
            sent: "".to_string(),
            data: hex::encode(&data.data),
            network_addr: data.network_addr.clone(),
        };

        info!("[{}] receive data {:?}", FN_NAME, push_data);

        {
            let mut mutex = self.queue_dldata.lock().unwrap();
            if !(*mutex).contains_key(addr) {
                (*mutex).insert(addr.clone(), VecDeque::<DlData>::new());
            }
            (*mutex).get_mut(addr).unwrap().push_back(push_data);
        }

        let result = NetDlDataResult {
            data_id: data.data_id,
            status: -1,
            message: None,
        };

        if let Err(e) = mgr.send_dldata_result(&result) {
            error!("[{}] send result {} error: {}", FN_NAME, result.data_id, e);
        }

        Ok(())
    }

    async fn on_ctrl(&self, _mgr: &NetworkMgr, _data: Box<NetworkCtrlMsg>) -> Result<(), ()> {
        const FN_NAME: &'static str = "MgrHandler::on_ctrl";

        info!("[{}] receive data", FN_NAME);

        Ok(())
    }
}
