# lora-ifroglab

A simple LoRa network server implementation for Sylvia-IoT using iFrogLab LoRa USB dongle devices.

This project is used for demo only. No authorization within.

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

## RX/TX rules

The gateway:
- Normally RX.
- Send node downlink data only after the gateway receives an uplink data from the node.

The node:
- Change to RX mode just after sending one uplink data.
- The interval of two uplink data should at least one second for receiving its downlink data.
