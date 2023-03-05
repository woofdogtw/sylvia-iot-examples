use serde::Serialize;

pub mod config;
pub mod data_handler;

#[derive(Clone, Debug, Serialize)]
pub struct UlData {
    pub time: String,
    #[serde(rename = "pub")]
    pub publish: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    pub rssi: Option<i16>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DlData {
    #[serde(rename = "correlationId")]
    pub correlation_id: String,
    #[serde(rename = "dataId", skip_serializing_if = "Option::is_none")]
    pub data_id: Option<String>,
    pub time: String,
    #[serde(rename = "networkCode")]
    pub network_code: String,
    #[serde(rename = "networkAddr")]
    pub network_addr: String,
    pub data: String,
    pub status: i32,
    pub error: Option<String>,
    pub message: Option<String>,
}

pub const MAX_DATA: usize = 100;
