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
}

pub const DEF_UNIT: &'static str = "test";
pub const DEF_CODE: &'static str = "app-demo";
pub const DEF_MQ_URI: &'static str = "amqp://guest:guest@localhost/application.test.app-demo";

/// To register Clap arguments.
pub fn reg_args(cmd: Command) -> Command {
    cmd.arg(
        Arg::new("app-demo.unit")
            .long("app-demo.unit")
            .help("app-demo unit code")
            .num_args(1)
            .default_value(DEF_UNIT),
    )
    .arg(
        Arg::new("app-demo.code")
            .long("app-demo.code")
            .help("app-demo application code")
            .num_args(1)
            .default_value(DEF_CODE),
    )
    .arg(
        Arg::new("app-demo.mq-uri")
            .long("app-demo.mq-uri")
            .help("app-demo message queue URI")
            .num_args(1)
            .default_value(DEF_MQ_URI),
    )
}

/// To read input arguments from command-line arguments and environment variables.
///
/// This function will call [`apply_default()`] to fill missing values so you do not need call it
/// again.
pub fn read_args(args: &ArgMatches) -> Config {
    apply_default(&Config {
        unit: match args.get_one::<String>("app-demo.unit") {
            None => match env::var("APP_DEMO_UNIT") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        code: match args.get_one::<String>("app-demo.code") {
            None => match env::var("APP_DEMO_CODE") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
        },
        mq_uri: match args.get_one::<String>("app-demo.mq-uri") {
            None => match env::var("APP_DEMO_MQ_URI") {
                Err(_) => None,
                Ok(v) => Some(v),
            },
            Some(v) => Some(v.clone()),
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
    }
}
