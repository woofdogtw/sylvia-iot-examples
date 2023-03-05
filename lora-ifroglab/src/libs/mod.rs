use serde::Serialize;

pub mod config;
pub mod data_handler;
pub mod lora_task;
pub mod lora_usb;

#[derive(Clone, Debug, Serialize)]
pub struct UlData {
    pub time: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    pub extension: UlDataExt,
}

#[derive(Clone, Debug, Serialize)]
pub struct UlDataExt {
    pub rssi: i16,
}

#[derive(Clone, Debug, Serialize)]
pub struct DlData {
    #[serde(skip_serializing)]
    pub data_id: String,
    pub time: String,
    #[serde(rename = "pub")]
    pub publish: String,
    pub sent: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
}

const MAX_DATA: usize = 100;
