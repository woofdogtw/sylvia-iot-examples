use std::{error::Error as StdError, fs, time::Duration};

use clap::{Arg as ClapArg, Command};
use log::{self, error};
use serde::Deserialize;
use sylvia_iot_sdk::util::logger;
use tokio;

use dev_demo::libs::{self, dev_task::Options};

#[derive(Deserialize)]
struct AppConfig {
    log: logger::Config,
    #[serde(rename = "devDemo")]
    dev_demo: libs::config::Config,
}

const PROJ_NAME: &'static str = env!("CARGO_PKG_NAME");
const PROJ_VER: &'static str = env!("CARGO_PKG_VERSION");

#[tokio::main]
async fn main() -> std::io::Result<()> {
    const FN_NAME: &'static str = "main";

    let conf = match init_config() {
        Err(e) => {
            let conf = &logger::Config {
                ..Default::default()
            };
            logger::init(PROJ_NAME, &conf);
            error!("[{}] read config error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(conf) => conf,
    };

    logger::init(PROJ_NAME, &conf.log);

    let opts = Options {
        dev_path: conf.dev_demo.dev_path.unwrap(),
        freq: conf.dev_demo.freq.unwrap(),
        power: conf.dev_demo.power.unwrap(),
    };
    let _ = match libs::dev_task::DevTask::new(opts) {
        Err(e) => {
            error!("[{}] new task error: {}", FN_NAME, e);
            return Ok(());
        }
        Ok(task) => task,
    };
    loop {
        tokio::time::sleep(Duration::from_secs(1)).await
    }
}

fn init_config() -> Result<AppConfig, Box<dyn StdError>> {
    let mut args = Command::new(PROJ_NAME).version(PROJ_VER).arg(
        ClapArg::new("file")
            .short('f')
            .long("file")
            .help("config file")
            .num_args(1),
    );
    args = logger::reg_args(args);
    args = libs::config::reg_args(args);
    let args = args.get_matches();

    if let Some(v) = args.get_one::<String>("file") {
        let conf_str = fs::read_to_string(v)?;
        return Ok(json5::from_str(conf_str.as_str())?);
    }

    Ok(AppConfig {
        log: logger::read_args(&args),
        dev_demo: libs::config::read_args(&args),
    })
}
