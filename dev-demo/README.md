# dev-demo

A simple LoRa device that will send uplink data with the following data:

- Temperature
- Humidity
- Pressure

## Hardware

- Board: Raspberry Pi 4 Model B with [Waveshare Sense HAT(B)](https://www.waveshare.net/wiki/Sense_HAT_(B))
- LoRa module: [iFrogLab LoRa USB dongle](https://www.ifroglab.com/en/?product=ifroglab-lora-usb)

# Protocol

iFrogLab LoRa USB dongle can send 16 bytes data each time. `lora-ifroglab` defines the following fields:

    +---------------------------------------------------------+
    | Node Address |  (Reserved)  |          Payload          |
    +---------------------------------------------------------+
        4 bytes        4 bytes               8 bytes

- Node address: the unique address which can be received by using command **0x00**.
    - From node to gateway: the node address in every uplink data.
    - From gateway to node:
        - Node address for unicast data to the specified node.
        - **0x00000000** is used for broadcast data.
- (Reserved): used for future use. Must be zero.
- Payload: variable length payload. Can be zero bytes.

## Device Payload

We use 7 bytes to transfer device data (big-endian):

- `byte[1:0]`: temperature.
    - Temperature in Celsius: (175 * value) / 65536 - 45
- `byte[3:2]`: humidity.
    - Humidity in percentage: (100 * value) / 65536
- `byte[6:4]`: pressure.
    - Pressure in hPa: value / 4096
