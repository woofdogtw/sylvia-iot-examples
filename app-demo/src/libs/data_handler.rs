//! Implements data handlers for application data from the broker.

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

use async_trait::async_trait;
use log::{info, warn};
use sylvia_iot_sdk::{
    mq::{
        MgrStatus,
        application::{
            ApplicationMgr, DlDataResp as AppDlDataResp, DlDataResult as AppDlDataResult,
            EventHandler, UlData as AppUlData,
        },
    },
    util::strings,
};

use super::{DlData, MAX_DATA, UlData};

#[derive(Clone)]
pub struct MgrHandler {
    latest_uldata: Arc<Mutex<VecDeque<UlData>>>,
    latest_dldata: Arc<Mutex<VecDeque<DlData>>>,
}

impl MgrHandler {
    pub fn new(
        latest_uldata: Arc<Mutex<VecDeque<UlData>>>,
        latest_dldata: Arc<Mutex<VecDeque<DlData>>>,
    ) -> Self {
        MgrHandler {
            latest_uldata,
            latest_dldata,
        }
    }
}

#[async_trait]
impl EventHandler for MgrHandler {
    async fn on_status_change(&self, _mgr: &ApplicationMgr, _status: MgrStatus) {}

    async fn on_uldata(&self, _mgr: &ApplicationMgr, data: Box<AppUlData>) -> Result<(), ()> {
        const FN_NAME: &'static str = "MgrHandler::on_uldata";

        let data = UlData {
            time: strings::time_str(&data.time),
            publish: strings::time_str(&data.publish),
            network_code: data.network_code,
            network_addr: data.network_addr,
            data: hex::encode(&data.data),
            rssi: match data.extension {
                None => None,
                Some(ext) => match ext.get("rssi") {
                    None => None,
                    Some(value) => match value.as_i64() {
                        None => None,
                        Some(value) => Some(value as i16),
                    },
                },
            },
        };

        info!("[{}] receive data {:?}", FN_NAME, data);

        {
            let mut mutex = self.latest_uldata.lock().unwrap();
            (*mutex).push_back(data);
            if (*mutex).len() > MAX_DATA {
                (*mutex).pop_front();
            }
        }
        Ok(())
    }

    async fn on_dldata_resp(
        &self,
        _mgr: &ApplicationMgr,
        data: Box<AppDlDataResp>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "MgrHandler::on_dldata_resp";

        info!(
            "[{}] receive correlation ID {}",
            FN_NAME, data.correlation_id
        );

        let mut found = false;
        {
            let mut mutex = self.latest_dldata.lock().unwrap();
            for vec_data in (*mutex).iter_mut() {
                if vec_data.correlation_id.eq(data.correlation_id.as_str()) {
                    vec_data.data_id = data.data_id;
                    vec_data.error = data.error;
                    vec_data.message = data.message;
                    found = true;
                    break;
                }
            }
        }
        if !found {
            warn!(
                "[{}] no data for correletion ID {}",
                FN_NAME, data.correlation_id
            );
        }
        Ok(())
    }

    async fn on_dldata_result(
        &self,
        _mgr: &ApplicationMgr,
        data: Box<AppDlDataResult>,
    ) -> Result<(), ()> {
        const FN_NAME: &'static str = "MgrHandler::on_dldata_result";

        info!("[{}] receive data ID {}", FN_NAME, data.data_id);

        let mut found = false;
        {
            let mut mutex = self.latest_dldata.lock().unwrap();
            for vec_data in (*mutex).iter_mut() {
                if let Some(data_id) = vec_data.data_id.as_ref() {
                    if data_id.eq(data.data_id.as_str()) {
                        vec_data.status = data.status;
                        vec_data.message = data.message;
                        found = true;
                        break;
                    }
                }
            }
        }
        if !found {
            warn!("[{}] no data for data ID {}", FN_NAME, data.data_id);
        }
        Ok(())
    }
}
