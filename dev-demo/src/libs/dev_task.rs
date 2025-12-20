//! Device task handles device data collection and uses LoRa to send data:
//! - Use a timer task to collect device sensor data.
//! - Switch to TX mode and send data to the LoRa USB UART.
//! - Switch back to RX mode immediately to receive downlink data.

use std::{
    error::Error as StdError,
    io::{Error as IoError, ErrorKind},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::Utc;
use i2cdev::{core::I2CDevice, linux::LinuxI2CDevice};
use log::{debug, error, info, warn};
use tokio::{
    task::{self, JoinHandle},
    time,
};

use super::lora_usb::{ChipInfo, IfroglabLora};

pub struct Options {
    pub dev_path: String,
    pub freq: u32,
    pub power: u8,
}

#[derive(Clone)]
pub struct DevTask {
    opts: OptionsInner,

    task_handle: Arc<Mutex<Option<JoinHandle<()>>>>,

    shtc3_dev: Arc<Mutex<LinuxI2CDevice>>, // SHTC3 (temperature and humidity sensor)
    lps22hb_dev: Arc<Mutex<LinuxI2CDevice>>, // LPS22HB (barometric pressure sensor)
}

#[derive(Clone)]
struct OptionsInner {
    dev_path: String,
    freq: u32,
    power: u8,
}

struct LoraData {
    node_id: u32,
    payload: Vec<u8>,
}

const SLEEP_IDLE_MS: u64 = 100;
const I2C_DEV: &'static str = "/dev/i2c-1";
const I2C_SHTC3_ADDR: u16 = 0x70;
const I2C_LPS22HB_ADDR: u16 = 0x5c;

impl DevTask {
    pub fn new(opts: Options) -> Result<Self, Box<dyn StdError>> {
        const FN_NAME: &'static str = "DevTask::new";

        let mut shtc3_dev = match LinuxI2CDevice::new(I2C_DEV, I2C_SHTC3_ADDR) {
            Err(e) => {
                error!("[{}] new SHTC3 device error: {}", FN_NAME, e);
                return Err(Box::new(e));
            }
            Ok(dev) => dev,
        };
        reset_shtc3(&mut shtc3_dev)?;
        info!("[{}] SHTC3 initialized", FN_NAME);

        let mut lps22hb_dev = match LinuxI2CDevice::new(I2C_DEV, I2C_LPS22HB_ADDR) {
            Err(e) => {
                error!("[{}] new LPS22HB device error: {}", FN_NAME, e);
                return Err(Box::new(e));
            }
            Ok(dev) => dev,
        };
        reset_lps22hb(&mut lps22hb_dev)?;
        info!("[{}] LPS22HB initialized", FN_NAME);

        let task = DevTask {
            opts: OptionsInner {
                dev_path: opts.dev_path,
                freq: opts.freq,
                power: opts.power,
            },
            task_handle: Arc::new(Mutex::new(None)),
            shtc3_dev: Arc::new(Mutex::new(shtc3_dev)),
            lps22hb_dev: Arc::new(Mutex::new(lps22hb_dev)),
        };
        {
            *task.task_handle.lock().unwrap() = Some(create_event_loop(task.clone()));
        }
        Ok(task)
    }
}

/// To create an event loop runtime task.
fn create_event_loop(task: DevTask) -> JoinHandle<()> {
    task::spawn(async move {
        const FN_NAME: &'static str = "event_loop";
        // Main loop.
        let (mut port, mut chip_info, mut counter) = connect_port(&task).await;
        let mut port_connected = true;
        loop {
            if !port_connected {
                port_connected = true;
                (port, chip_info, counter) = connect_port(&task).await;
            }
            let sleep_time = 1000 - ((Utc::now().timestamp_millis() + 1) % 1000);
            time::sleep(Duration::from_millis(sleep_time as u64)).await;

            // Read before send.
            counter = match port.cmd07_read_data_counter().await {
                Err(e) => {
                    error!("[{}] get counter error: {}", FN_NAME, e);
                    port_connected = false;
                    continue;
                }
                Ok(new_counter) => {
                    if counter != new_counter {
                        let read_data = match port.cmd06_read_data().await {
                            Err(e) => {
                                error!("[{}] get counter error: {}", FN_NAME, e);
                                port_connected = false;
                                continue;
                            }
                            Ok(data) => match data {
                                None => continue,
                                Some(data) => data,
                            },
                        };
                        let rx_data = match parse_rx_data(read_data.data.as_slice()) {
                            Err(e) => {
                                warn!("[{}] get counter error: {}", FN_NAME, e);
                                continue;
                            }
                            Ok(data) => data,
                        };
                        if rx_data.node_id == chip_info.node_id {
                            info!(
                                "[{}] get data from node {:08x?} and data is {}, RSSI: {}",
                                FN_NAME,
                                rx_data.node_id,
                                hex::encode(&rx_data.payload),
                                read_data.rssi
                            );
                        } else if rx_data.node_id == 0 {
                            info!(
                                "[{}] get broadcast data: {}, RSSI: {}",
                                FN_NAME,
                                hex::encode(&rx_data.payload),
                                read_data.rssi
                            );
                        } else {
                            debug!("[{}] skip other device data", FN_NAME);
                        }
                    }
                    new_counter
                }
            };

            // Read sensor data.
            let (temp, humid) = match read_temp_humid(&task).await {
                Err(_) => continue,
                Ok(v) => (v.0, v.1),
            };
            let (pressure, temp2) = match read_pressure_temp(&task).await {
                Err(_) => continue,
                Ok(v) => (v.0, v.1),
            };
            info!(
                "[{}] read sensor data. temperature: {} C, humidity: {}%, pressure: {} hPa, temperature: {} C",
                FN_NAME,
                (175_u32 * temp as u32) / 65536 - 45,
                (100_u32 * humid as u32) / 65536,
                pressure / 4096,
                temp2 / 100,
            );

            let mut buff = [0u8; 16];
            buff[0..4].clone_from_slice(&chip_info.node_id.to_be_bytes());
            buff[11..15].clone_from_slice(&pressure.to_be_bytes()); // write 32-bit and overwritten by humidity
            buff[8..10].clone_from_slice(&temp.to_be_bytes());
            buff[10..12].clone_from_slice(&humid.to_be_bytes());
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
                    port_connected = false;
                }
                continue;
            }
            if let Err(e) = port.cmd05_write_data(&buff[..15]).await {
                error!("[{}] send cmd05 error: {}", FN_NAME, e);
                if let Err(e) = port
                    .cmd03_set_values(3, task.opts.freq, task.opts.power)
                    .await
                {
                    error!("[{}] set back RX mode error: {}", FN_NAME, e);
                    port_connected = false;
                }
                continue;
            }
            if let Err(e) = port
                .cmd03_set_values(3, task.opts.freq, task.opts.power)
                .await
            {
                error!("[{}] set back RX mode error: {}", FN_NAME, e);
                port_connected = false;
            }
        }
    })
}

fn parse_rx_data(raw: &[u8]) -> Result<LoraData, IoError> {
    if raw.len() < 8 {
        return Err(IoError::from(ErrorKind::InvalidData));
    }

    let mut dst = [0u8; 4];
    dst.clone_from_slice(&raw[0..4]);
    Ok(LoraData {
        node_id: u32::from_be_bytes(dst),
        payload: raw[8..].to_vec(),
    })
}

async fn connect_port(task: &DevTask) -> (IfroglabLora, ChipInfo, u16) {
    const FN_NAME: &'static str = "connect_port";

    loop {
        time::sleep(Duration::from_millis(SLEEP_IDLE_MS)).await;
        let mut port = match IfroglabLora::new(task.opts.dev_path.as_str()) {
            Err(e) => {
                error!("[{}] create port error: {}", FN_NAME, e);
                continue;
            }
            Ok(port) => port,
        };
        let chip_info = match port.cmd00_chip_info().await {
            Err(e) => {
                error!("[{}] read chip info error: {}", FN_NAME, e);
                continue;
            }
            Ok(info) => info,
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
        info!("[{}] connected to port", FN_NAME);
        break (port, chip_info, counter);
    }
}

fn reset_shtc3(dev: &mut LinuxI2CDevice) -> Result<(), Box<dyn StdError>> {
    const FN_NAME: &'static str = "reset_shtc3";

    if let Err(e) = dev.write(&[0x40_u8, 0x1a_u8]) {
        error!("[{}] reset 0x401a error: {}", FN_NAME, e);
        return Err(Box::new(e));
    }

    Ok(())
}

fn reset_lps22hb(dev: &mut LinuxI2CDevice) -> Result<(), Box<dyn StdError>> {
    const FN_NAME: &'static str = "reset_lps22hb";

    let buf = match dev.smbus_read_word_data(0x11) {
        Err(e) => {
            error!("[{}] read word 0x11 error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }
        Ok(buf) => buf as u8,
    };
    if let Err(e) = dev.smbus_write_byte_data(0x11, buf | 0x04) {
        error!("[{}] write byte 0x11 error: {}", FN_NAME, e);
        return Err(Box::new(e));
    }
    loop {
        let buf = match dev.smbus_read_word_data(0x11) {
            Err(e) => {
                error!("[{}] read word 0x11 error: {}", FN_NAME, e);
                return Err(Box::new(e));
            }
            Ok(buf) => buf as u8,
        };
        if buf & 0x04 == 0 {
            break;
        }
    }

    Ok(())
}

/// Read SHTC3 (temperature and humidity sensor) values.
///
/// The returned tuple contains (temperature, humidity) values.
/// - Temperature in Celsius: (175 * value) / 65536 - 45
/// - Humidity in percentage: (100 * value) / 65536
async fn read_temp_humid(task: &DevTask) -> Result<(u16, u16), Box<dyn StdError>> {
    const FN_NAME: &'static str = "read_temp_humid";

    let dev = task.shtc3_dev.clone();
    let (temp, humid) = match task::spawn_blocking(move || {
        let mut dev = dev.lock().unwrap();

        // Read temperature.
        if let Err(e) = dev.write(&[0x78_u8, 0x66_u8]) {
            error!(
                "[{}] write temperature command 0x7866 error: {}",
                FN_NAME, e
            );
            return Err(Box::new(e));
        }
        std::thread::sleep(Duration::from_millis(20));
        let mut buf = [0u8; 3];
        if let Err(e) = dev.read(&mut buf) {
            error!("[{}] read temperature error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }
        let temp: u16 = buf[0] as u16 * 256 + buf[1] as u16;

        // Read humidity.
        if let Err(e) = dev.write(&[0x58_u8, 0xe0_u8]) {
            error!("[{}] write humidity command 0x58e0 error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }
        std::thread::sleep(Duration::from_millis(20));
        let mut buf = [0u8; 3];
        if let Err(e) = dev.read(&mut buf) {
            error!("[{}] read humidity error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }
        let humid: u16 = buf[0] as u16 * 256 + buf[1] as u16;

        Ok((temp, humid))
    })
    .await
    {
        Err(e) => {
            error!("[{}] run spawn_blocking error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }
        Ok(ret) => ret?,
    };

    Ok((temp, humid))
}

/// Read LPS22HB (barometric pressure sensor) values.
///
/// The returned tuple contains (pressure, temperature) values.
/// - Pressure in hPa: value / 4096
/// - Temperature in Celsius: value / 100
async fn read_pressure_temp(task: &DevTask) -> Result<(u32, u32), Box<dyn StdError>> {
    const FN_NAME: &'static str = "read_pressure_temp";

    let dev = task.lps22hb_dev.clone();
    let (pressure, temp) = match task::spawn_blocking(move || {
        let mut dev = dev.lock().unwrap();

        // Trigger one shot data acquisition.
        let buf = match dev.smbus_read_word_data(0x11) {
            Err(e) => {
                error!("[{}] trigger one shot read 0x11 error: {}", FN_NAME, e);
                return Err(Box::new(e));
            }
            Ok(buf) => buf as u8,
        };
        if let Err(e) = dev.smbus_write_byte_data(0x11, buf | 0x01) {
            error!("[{}] trigger one shot write 0x11 error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }

        // Read pressure data.
        let pressure = match dev.smbus_read_byte_data(0x27) {
            Err(e) => {
                error!("[{}] read pressure 0x27 error: {}", FN_NAME, e);
                return Err(Box::new(e));
            }
            Ok(buf) => match buf & 0x01 {
                0x01 => {
                    let out_xl = match dev.smbus_read_byte_data(0x28) {
                        Err(e) => {
                            error!("[{}] read pressure 0x28 error: {}", FN_NAME, e);
                            return Err(Box::new(e));
                        }
                        Ok(xl) => xl,
                    };
                    let out_l = match dev.smbus_read_byte_data(0x29) {
                        Err(e) => {
                            error!("[{}] read pressure 0x29 error: {}", FN_NAME, e);
                            return Err(Box::new(e));
                        }
                        Ok(l) => l,
                    };
                    let out_h = match dev.smbus_read_byte_data(0x2a) {
                        Err(e) => {
                            error!("[{}] read pressure 0x2a error: {}", FN_NAME, e);
                            return Err(Box::new(e));
                        }
                        Ok(h) => h,
                    };
                    out_h as u32 * 65536 + out_l as u32 * 256 + out_xl as u32
                }
                _ => 0_u32,
            },
        };

        // Read temperature data.
        let temp = match dev.smbus_read_byte_data(0x27) {
            Err(e) => {
                error!("[{}] read temperature 0x27 error: {}", FN_NAME, e);
                return Err(Box::new(e));
            }
            Ok(buf) => match buf & 0x02 {
                0x02 => {
                    let out_l = match dev.smbus_read_byte_data(0x2b) {
                        Err(e) => {
                            error!("[{}] read temperature 0x2b error: {}", FN_NAME, e);
                            return Err(Box::new(e));
                        }
                        Ok(l) => l,
                    };
                    let out_h = match dev.smbus_read_byte_data(0x2c) {
                        Err(e) => {
                            error!("[{}] read temperature 0x2c error: {}", FN_NAME, e);
                            return Err(Box::new(e));
                        }
                        Ok(h) => h,
                    };
                    out_h as u32 * 256 + out_l as u32
                }
                _ => 0_u32,
            },
        };

        Ok((pressure, temp))
    })
    .await
    {
        Err(e) => {
            error!("[{}] run spawn_blocking error: {}", FN_NAME, e);
            return Err(Box::new(e));
        }
        Ok(ret) => ret?,
    };

    Ok((pressure, temp))
}
