//! iFrogLab USB dongle serial port operations.

use std::{
    io::{Error as IoError, ErrorKind},
    time::Duration,
};

use chrono::{DateTime, Utc};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    time,
};
use tokio_serial::{SerialPortBuilderExt, SerialStream};

/// Provides functions to control iFrogLab LoRa USB dongle.
pub struct IfroglabLora {
    port: SerialStream,
    buff: [u8; 24],
}

/// Chip information from command 0x00.
pub struct ChipInfo {
    pub fw_ver: u8,
    pub chip_id: u8,
    pub node_id: u32,
}

/// Current chip values.
pub struct ChipValues {
    /// Operation mode.
    /// - 0: sleep
    /// - 1: standby
    /// - 2: TX
    /// - 3: RX
    pub mode: u8,
    /// Frequency of 10,000 Hz. For example, 91500 is 915.00 MHz.
    pub freq: u32,
    /// Power level. Level 0~15 means 2~17 dBm.
    pub power: u8,
    /// Bandwidth.
    /// - 1: 125k
    /// - 2: 250k
    /// - 3: 500k (default)
    pub bw: u8,
    /// Code rate.
    /// - 1: 4/5 (default)
    /// - 2: 4/6
    /// - 3: 4/7
    /// - 4: 4/8
    pub cr: u8,
    /// Spreading factor.
    /// - 1: 6
    /// - 2: 7
    /// - 3: 8
    /// - 4: 9 (default)
    /// - 5: 10
    /// - 6: 11
    /// - 7: 12
    pub sf: u8,
}

pub struct ReadData {
    pub data: Vec<u8>,
    pub rssi: i16,
}

const TIMEOUT_MS: i64 = 1000;
const ACK_SLEEP_MS: u64 = 20;

impl IfroglabLora {
    /// Create a port stream for the USB dongle device.
    pub fn new(path: &str) -> Result<Self, IoError> {
        let port = tokio_serial::new(path, 115200)
            .timeout(Duration::from_secs(2))
            .open_native_async()?;

        Ok(IfroglabLora {
            port,
            buff: [0u8; 24],
        })
    }

    pub async fn cmd00_chip_info(&mut self) -> Result<ChipInfo, IoError> {
        let cmd: [u8; 4] = [0x80, 0, 0, 0x80];
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() < 2 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "cmd-00 should with at least 2 bytes",
            ));
        }

        let mut node_id = 0;
        if data[1] >= 8 && data.len() >= 6 {
            let mut dst = [0u8; 4];
            dst.clone_from_slice(&data[2..6]);
            node_id = u32::from_be_bytes(dst);
        }

        Ok(ChipInfo {
            fw_ver: data[1],
            chip_id: data[0],
            node_id,
        })
    }

    pub async fn cmd01_reset(&mut self) -> Result<(), IoError> {
        let cmd: [u8; 4] = [0xC1, 0x01, 0, 0xC0];
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() != 1 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "cmd-01 should with 1 byte",
            ));
        } else if data[0] != 0x55 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("cmd-01 not response 0x55: 0x{:02x?}", data[0]),
            ));
        }

        Ok(())
    }

    pub async fn cmd02_get_chip_values(&mut self) -> Result<ChipValues, IoError> {
        let cmd: [u8; 4] = [0xC1, 0x02, 0, 0xC3];
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() != 8 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "cmd-02 should with 8 bytes",
            ));
        }

        let mut dst = [0u8; 4];
        (&mut dst[1..]).clone_from_slice(&data[1..4]);
        let freq = u32::from_be_bytes(dst);

        Ok(ChipValues {
            mode: data[0],
            freq,
            power: data[4],
            bw: data[5],
            cr: data[6],
            sf: data[7],
        })
    }

    pub async fn cmd03_set_values(
        &mut self,
        mut mode: u8,
        mut freq: u32,
        mut power: u8,
    ) -> Result<(), IoError> {
        if mode > 3 {
            mode = 1;
        }
        if freq < 86000 || freq > 102000 {
            freq = 91500;
        }
        if power > 15 {
            power = 0;
        }

        let mut cmd: [u8; 9] = [0xC1, 0x03, 0x05, mode, 0, 0, 0, power, 0];
        cmd[4] = ((freq >> 16) & 0xff) as u8;
        cmd[5] = ((freq >> 8) & 0xff) as u8;
        cmd[6] = (freq & 0xff) as u8;
        cmd[8] = crc(&cmd);
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() != 1 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "cmd-03 should with 1 byte",
            ));
        } else if data[0] != 0x55 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("cmd-03 not response 0x55: 0x{:02x?}", data[0]),
            ));
        }

        Ok(())
    }

    pub async fn cmd04_set_values(
        &mut self,
        mut bw: u8,
        mut cr: u8,
        mut sf: u8,
    ) -> Result<(), IoError> {
        if bw < 1 || bw > 3 {
            bw = 3;
        }
        if cr < 1 || cr > 4 {
            cr = 1;
        }
        if sf < 1 || sf > 7 {
            sf = 3;
        }

        let mut cmd: [u8; 7] = [0xC1, 0x04, 0x03, bw, cr, sf, 0];
        cmd[6] = crc(&cmd);
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() != 1 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "cmd-04 should with 1 byte",
            ));
        } else if data[0] != 0x55 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("cmd-04 not response 0x55: 0x{:02x?}", data[0]),
            ));
        }

        Ok(())
    }

    pub async fn cmd05_write_data(&mut self, data: &[u8]) -> Result<(), IoError> {
        if data.len() < 1 || data.len() > 16 {
            return Err(IoError::from(ErrorKind::InvalidInput));
        }

        let mut cmd = [0u8; 20];
        let len = data.len() as u8;
        cmd[0] = 0xC1;
        cmd[1] = 0x05;
        cmd[2] = len;
        (&mut cmd[3..((len + 3) as usize)]).clone_from_slice(data);
        cmd[(len + 3) as usize] = crc(&cmd);
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd[..((len + 4) as usize)]).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() != 1 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                "cmd-04 should with 1 byte",
            ));
        } else if data[0] != 0x55 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("cmd-04 not response 0x55: 0x{:02x?}", data[0]),
            ));
        }

        Ok(())
    }

    pub async fn cmd06_read_data(&mut self) -> Result<Option<ReadData>, IoError> {
        let cmd: [u8; 4] = [0xC1, 0x06, 0, 0xC7];
        let data;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            data = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => data,
            };
            break;
        }
        if data.len() == 0 {
            return Ok(None);
        } else if data.len() < 5 {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("cmd-06 should with at least 5 bytes, only {}", data.len()),
            ));
        }

        let mut rssi: i16 = 0;
        let mut get_data = &data[..];
        if data.len() > 2 {
            get_data = &data[..(data.len() - 2)];
            let mut dst = [0u8; 2];
            dst.clone_from_slice(&data[(data.len() - 2)..data.len()]);
            rssi = i16::from_be_bytes(dst);
        }

        Ok(Some(ReadData {
            data: Vec::from(get_data),
            rssi,
        }))
    }

    pub async fn cmd07_read_data_counter(&mut self) -> Result<u16, IoError> {
        let cmd: [u8; 4] = [0xC1, 0x07, 0, 0xC6];
        let counter;
        let start = Utc::now();
        loop {
            self.port.write(&cmd).await?;
            counter = match self.read_ack().await {
                Err(e) => {
                    not_timeout(start, e)?;
                    continue;
                }
                Ok(data) => {
                    let mut dst = [0u8; 2];
                    dst.clone_from_slice(data);
                    u16::from_be_bytes(dst)
                }
            };
            break;
        }

        Ok(counter)
    }

    /// Read ACK for the command from BYTE-4 (skip the first 3 bytes) with `len` size.
    async fn read_ack(&mut self) -> Result<&[u8], IoError> {
        time::sleep(Duration::from_millis(ACK_SLEEP_MS)).await;

        // Read buffer and get the `len` field.
        let mut size = self.port.read(&mut self.buff).await?;
        if size < 3 {
            // Second chance.
            time::sleep(Duration::from_millis(ACK_SLEEP_MS)).await;
            size = size + self.port.read(&mut self.buff[size..]).await?;
            if size < 3 {
                return Err(IoError::new(
                    ErrorKind::TimedOut,
                    format!("less than 3 bytes, only {} bytes", size),
                ));
            }
        }

        // Check if data length will be larger than reserved buffer size.
        let len = self.buff[2] as usize;
        if len + 4 > self.buff.len() {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("invalid `len` field: {}, buff: {:?}", len, self.buff),
            ));
        }

        // Check if data is received completely.
        if size < len + 4 {
            // Second chance.
            time::sleep(Duration::from_millis(ACK_SLEEP_MS)).await;
            size = size + self.port.read(&mut self.buff[size..]).await?;
            if size < len {
                return Err(IoError::new(
                    ErrorKind::TimedOut,
                    format!("only receive {}/{} bytes", size, len),
                ));
            }
        }

        // Check CRC.
        let crc = crc(&self.buff[..len + 3]);
        if crc != self.buff[len + 3] {
            return Err(IoError::new(
                ErrorKind::InvalidData,
                format!("invalid CRC: {:?}, size: {}", self.buff, size),
            ));
        }

        if self.buff[1] == 0xff {
            return Err(IoError::new(
                ErrorKind::Other,
                format!("{:02x?}", self.buff[3]),
            ));
        }

        Ok(&self.buff[3..len + 3])
    }
}

/// Calculate CRC.
fn crc(data: &[u8]) -> u8 {
    let mut result: u8 = 0;
    for d in data {
        result ^= d;
    }
    result
}

fn not_timeout(start: DateTime<Utc>, err: IoError) -> Result<(), IoError> {
    match Utc::now().timestamp_millis() - start.timestamp_millis() > TIMEOUT_MS {
        false => Ok(()),
        true => Err(err),
    }
}
