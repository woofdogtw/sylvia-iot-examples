//! LoRa task handles LoRa gateway RX operations:
//! - Use a timer task to poll new RX data.
//! - Send the new RX data as an uplink data to the `uldata` queue.
//! - Pop one queued downlink data and send TX data to the device.

use std::{
    collections::{HashMap, VecDeque},
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use hex;
use log::{error, info, warn};
use serde_json::{json, Map};
use sylvia_iot_sdk::{
    mq::network::{DlDataResult, NetworkMgr, UlData as NetUlData},
    util::strings,
};
use tokio::{
    task::{self, JoinHandle},
    time,
};

use super::{lora_usb::IfroglabLora, DlData, UlData, UlDataExt, MAX_DATA};

pub struct Options {
    pub mgr: Arc<Mutex<NetworkMgr>>,
    pub latest_uldata: Arc<Mutex<VecDeque<UlData>>>,
    pub latest_dldata: Arc<Mutex<VecDeque<DlData>>>,
    pub queue_dldata: Arc<Mutex<HashMap<String, VecDeque<DlData>>>>,
    pub dev_path: String,
    pub freq: u32,
    pub power: u8,
}

#[derive(Clone)]
pub struct LoraTask {
    opts: OptionsInner,

    queue_rsc: QueueRsc,
    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,
}

#[derive(Clone)]
struct QueueRsc {
    mgr: Arc<Mutex<NetworkMgr>>,
    latest_uldata: Arc<Mutex<VecDeque<UlData>>>,
    latest_dldata: Arc<Mutex<VecDeque<DlData>>>,
    queue_dldata: Arc<Mutex<HashMap<String, VecDeque<DlData>>>>,
}

#[derive(Clone)]
struct OptionsInner {
    dev_path: String,
    freq: u32,
    power: u8,
}

struct RxData {
    node_id: u32,
    payload: Vec<u8>,
}

const SLEEP_IDLE_MS: u64 = 100;

impl LoraTask {
    pub fn new(opts: Options) -> Result<Self, Box<dyn StdError>> {
        let queue_rsc = QueueRsc {
            mgr: opts.mgr,
            latest_uldata: opts.latest_uldata,
            latest_dldata: opts.latest_dldata,
            queue_dldata: opts.queue_dldata,
        };

        let task = LoraTask {
            opts: OptionsInner {
                dev_path: opts.dev_path,
                freq: opts.freq,
                power: opts.power,
            },
            queue_rsc,
            task_handle: Arc::new(Mutex::new(None)),
        };
        {
            *task.task_handle.lock().unwrap() = Some(create_event_loop(task.clone()));
        }
        Ok(task)
    }
}

/// To create an event loop runtime task.
fn create_event_loop(task: LoraTask) -> JoinHandle<()> {
    task::spawn(async move {
        const FN_NAME: &'static str = "event_loop";
        let sleep_time = SLEEP_IDLE_MS;
        // Connect to the USB dongle.
        let (mut port, mut counter) = loop {
            time::sleep(Duration::from_millis(SLEEP_IDLE_MS)).await;
            let mut port = match IfroglabLora::new(task.opts.dev_path.as_str()) {
                Err(e) => {
                    error!("[{}] create port error: {}", FN_NAME, e);
                    continue;
                }
                Ok(port) => port,
            };
            if let Err(e) = port
                .cmd03_set_values(3, task.opts.freq, task.opts.power)
                .await
            {
                error!("[{}] set RX mode error: {}", FN_NAME, e);
                continue;
            }
            let counter = match port.cmd07_read_data_counter().await {
                Err(e) => {
                    error!("[{}] get counter error: {}", FN_NAME, e);
                    continue;
                }
                Ok(counter) => counter,
            };
            break (port, counter);
        };
        info!("[{}] connected to port", FN_NAME);
        // Main loop.
        loop {
            time::sleep(Duration::from_millis(sleep_time)).await;
            counter = match port.cmd07_read_data_counter().await {
                Err(e) => {
                    error!("[{}] get counter error: {}", FN_NAME, e);
                    continue;
                }
                Ok(new_counter) => match counter == new_counter {
                    false => new_counter,
                    true => continue,
                },
            };
            let read_data = match port.cmd06_read_data().await {
                Err(e) => {
                    error!("[{}] get counter error: {}", FN_NAME, e);
                    continue;
                }
                Ok(data) => match data {
                    None => continue,
                    Some(data) => data,
                },
            };

            // Send uplink data to the broker.
            let rx_data = match parse_rx_data(read_data.data.as_slice()) {
                Err(e) => {
                    warn!("[{}] get counter error: {}", FN_NAME, e);
                    continue;
                }
                Ok(data) => data,
            };
            let addr = format!("{:08x?}", rx_data.node_id);
            let mut extension = Map::new();
            extension.insert("rssi".to_string(), json!(read_data.rssi));
            let uldata = NetUlData {
                time: Utc::now(),
                network_addr: addr.clone(),
                data: hex::encode(rx_data.payload.as_slice()),
                extension: Some(extension),
            };
            {
                let api_data = UlData {
                    time: strings::time_str(&uldata.time),
                    network_addr: addr.clone(),
                    data: uldata.data.clone(),
                    extension: UlDataExt {
                        rssi: read_data.rssi,
                    },
                };
                let mut mutex = task.queue_rsc.latest_uldata.lock().unwrap();
                (*mutex).push_back(api_data);
                if (*mutex).len() > MAX_DATA {
                    (*mutex).pop_front();
                }
            }
            {
                if let Err(e) = task.queue_rsc.mgr.lock().unwrap().send_uldata(&uldata) {
                    error!("[{}] send uldata message error: {}", FN_NAME, e);
                    continue;
                }
            }

            // Send one downlink data to the node if there are queued data to be send.
            let mut data = {
                let mut mutex = task.queue_rsc.queue_dldata.lock().unwrap();
                match (*mutex).get_mut(addr.as_str()) {
                    None => continue,
                    Some(queue) => match queue.pop_front() {
                        None => continue,
                        Some(data) => data,
                    },
                }
            };
            let mut buff = [0u8; 16];
            let data_len = data.data.len();
            if data_len > 16 {
                let result = DlDataResult {
                    data_id: data.data_id.clone(),
                    status: 1,
                    message: Some(format!("exceed 16-byte hexadecimal")),
                };
                {
                    if let Err(e) = task
                        .queue_rsc
                        .mgr
                        .lock()
                        .unwrap()
                        .send_dldata_result(&result)
                    {
                        error!("[{}] send result message error: {}", FN_NAME, e);
                        continue;
                    }
                }
            } else if let Err(e) =
                hex::decode_to_slice(data.data.as_str(), &mut buff[8..8 + data_len / 2])
            {
                error!(
                    "[{}] decode hexadecimal data error: {}, data: {}",
                    FN_NAME,
                    e,
                    data.data.as_str()
                );
                continue;
            }
            buff[0..4].clone_from_slice(&rx_data.node_id.to_be_bytes());
            if let Err(e) = port
                .cmd03_set_values(2, task.opts.freq, task.opts.power)
                .await
            {
                error!("[{}] set TX mode error: {}", FN_NAME, e);
                if let Err(e) = port
                    .cmd03_set_values(3, task.opts.freq, task.opts.power)
                    .await
                {
                    error!("[{}] set back RX mode error: {}", FN_NAME, e);
                }
                continue;
            }
            if let Err(e) = port.cmd05_write_data(&buff[..8 + data_len / 2]).await {
                error!("[{}] send cmd05 error: {}", FN_NAME, e);
                if let Err(e) = port
                    .cmd03_set_values(3, task.opts.freq, task.opts.power)
                    .await
                {
                    error!("[{}] set back RX mode error: {}", FN_NAME, e);
                }
                continue;
            }
            if let Err(e) = port
                .cmd03_set_values(3, task.opts.freq, task.opts.power)
                .await
            {
                error!("[{}] set back RX mode error: {}", FN_NAME, e);
            }
            data.sent = strings::time_str(&Utc::now());
            {
                let mut mutex = task.queue_rsc.latest_dldata.lock().unwrap();
                (*mutex).push_back(data);
                if (*mutex).len() > MAX_DATA {
                    (*mutex).pop_front();
                }
            }
        }
    })
}

fn parse_rx_data(raw: &[u8]) -> Result<RxData, IoError> {
    if raw.len() < 8 {
        return Err(IoError::from(ErrorKind::InvalidData));
    }

    let mut dst = [0u8; 4];
    dst.clone_from_slice(&raw[0..4]);
    Ok(RxData {
        node_id: u32::from_be_bytes(dst),
        payload: raw[8..].to_vec(),
    })
}
