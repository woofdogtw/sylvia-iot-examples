//! Program configurations.

use std::env;

use clap::{Arg, ArgMatches, Command};
use serde::Deserialize;

/// Configuration file object.
#[derive(Default, Deserialize)]
pub struct Config {
    pub unit: Option<String>,
    pub code: Option<String>,
    #[serde(rename = "mqUri")]
    pub mq_uri: Option<String>,
    /// Serial port device path such as `/dev/ttyACM0` or `COM1`.
    #[serde(rename = "devPath")]
    pub dev_path: Option<String>,
    pub freq: Option<u32>,
    pub power: Option<u8>,
}

pub const DEF_UNIT: &'static str = "test";
pub const DEF_CODE: &'static str = "lora-ifroglab";
pub const DEF_MQ_URI: &'static str = "amqp://guest:guest@localhost/network.test.lora-ifroglab";
pub const DEF_DEV_PATH: &'static str = "/dev/ttyACM0";
pub const DEF_FREQ: u32 = 91500;
pub const DEF_FREQ_STR: &'static str = "91500";
pub const DEF_POWER: u8 = 0;
pub const DEF_POWER_STR: &'static str = "0";

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("lora-ifroglab.unit")
            .long("lora-ifroglab.unit")
            .help("lora-ifroglab unit code")
            .num_args(1)
            .default_value(DEF_UNIT),
    )
    .arg(
        Arg::new("lora-ifroglab.code")
            .long("lora-ifroglab.code")
            .help("lora-ifroglab network code")
            .num_args(1)
            .default_value(DEF_CODE),
    )
    .arg(
        Arg::new("lora-ifroglab.mq-uri")
            .long("lora-ifroglab.mq-uri")
            .help("lora-ifroglab message queue URI")
            .num_args(1)
            .default_value(DEF_MQ_URI),
    )
    .arg(
        Arg::new("lora-ifroglab.dev-path")
            .long("lora-ifroglab.dev-path")
            .help("Device path such as `/dev/ttyACM0` or `COM1`")
            .num_args(1)
            .default_value(DEF_DEV_PATH),
    )
    .arg(
        Arg::new("lora-ifroglab.freq")
            .long("lora-ifroglab.freq")
            .help("Frequency (10kHz). 86000~102000")
            .num_args(1)
            .value_parser(86000..=102000)
            .default_value(DEF_FREQ_STR),
    )
    .arg(
        Arg::new("lora-ifroglab.power")
            .long("lora-ifroglab.power")
            .help("RF power. 0~15 for 2~17 dBm")
            .num_args(1)
            .value_parser(0..=15)
            .default_value(DEF_POWER_STR),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        unit: match args.get_one::<String>("lora-ifroglab.unit") {
            None => match env::var("LORA_IFROGLAB_UNIT") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        code: match args.get_one::<String>("lora-ifroglab.code") {
            None => match env::var("LORA_IFROGLAB_CODE") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        mq_uri: match args.get_one::<String>("lora-ifroglab.mq-uri") {
            None => match env::var("LORA_IFROGLAB_MQ_URI") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        dev_path: match args.get_one::<String>("lora-ifroglab.dev-path") {
            None => match env::var("LORA_IFROGLAB_DEV_PATH") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        freq: match args.get_one::<i64>("lora-ifroglab.freq") {
            None => match env::var("LORA_IFROGLAB_FREQ") {
                Err(_) => Some(DEF_FREQ),
                Ok(v) => match v.parse::<u32>() {
                    Err(_) => Some(DEF_FREQ),
                    Ok(v) => Some(v),
                },
            },
            Some(v) => Some(*v as u32),
        },
        power: match args.get_one::<i64>("lora-ifroglab.power") {
            None => match env::var("LORA_IFROGLAB_POWER") {
                Err(_) => Some(DEF_POWER),
                Ok(v) => match v.parse::<u8>() {
                    Err(_) => Some(DEF_POWER),
                    Ok(v) => Some(v),
                },
            },
            Some(v) => Some(*v as u8),
        },
    })
}

/// Fill missing configuration with default values.
pub fn apply_default(config: &Config) -> Config {
    Config {
        unit: match config.unit.as_ref() {
            None => Some(DEF_UNIT.to_string()),
            Some(unit) => Some(unit.clone()),
        },
        code: match config.code.as_ref() {
            None => Some(DEF_CODE.to_string()),
            Some(code) => Some(code.clone()),
        },
        mq_uri: match config.mq_uri.as_ref() {
            None => Some(DEF_MQ_URI.to_string()),
            Some(uri) => Some(uri.clone()),
        },
        dev_path: match config.dev_path.as_ref() {
            None => Some(DEF_CODE.to_string()),
            Some(path) => Some(path.clone()),
        },
        freq: match config.freq.as_ref() {
            None => Some(DEF_FREQ),
            Some(freq) => Some(freq.clone()),
        },
        power: match config.power.as_ref() {
            None => Some(DEF_POWER),
            Some(power) => Some(power.clone()),
        },
    }
}
