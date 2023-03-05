API - app-demo
==============

## Contents

- [Data APIs](#data)
    - [`GET /app-demo/api/v1/data/uldata` Get latest uplink data](#get_data_uldata)
    - [`GET /app-demo/api/v1/data/dldata` Get latest downlink data](#get_data_dldata)
    - [`POST /app-demo/api/v1/data/dldata` Send downlink data](#post_data_dldata)

# <a name="data"></a>Data APIs

## <a name="get_data_uldata"></a>Get latest uplink data

Get latest 100 uplink data from all nodes.

    GET /app-demo/api/v1/data/uldata

#### Response

- **200 OK**: Latest uplink data. Parameters are:

    - *object[]* `data`:
        - *string* `time`: Device time for this data in ISO 8601 format.
        - *string* `pub`: The publish time for this data in ISO 8601 format.
        - *string* `networkCode`: Network code.
        - *string* `networkAddr`: Network address.
        - *string* `data`: Payload data in hexadecimal string.
        - *number* `rssi`: (**optional**) The RSSI value of the data.

- **500, 503**: See [Notes](#notes).

## <a name="get_data_dldata"></a>Get latest downlink data

Get latest 100 downlink data from the application server.

    GET /app-demo/api/v1/data/dldata

#### Response

- **200 OK**: Latest downlink data. Parameters are:

    - *object[]* `data`:
        - *string* `deviceId`: (**optional**) The device ID that is accpeted by the broker.
        - *string* `time`: The received time from the queue in ISO 8601 format.
        - *string* `networkCode`: Network code.
        - *string* `networkAddr`: Network address.
        - *string* `data`: Payload data in hexadecimal string.
        - *number* `status`: **0** for success, negative for processing, positive for error.
        - *string* `error`: (**optional**) Error code.
        - *string* `message`: (**optional**) Detail message.

- **500, 503**: See [Notes](#notes).

## <a name="post_application_dldata"></a>Send downlink data

Send downlink data to a device.

    POST /app-demo/api/v1/data/dldata

#### Parameters

- *object* `data`: An object that contains the downlink data information.
    - *string* `networkCode`: The network code of the target device.
    - *string* `networkAddr`: The network address of the target device.
    - *string* `payload`: The data payload in **hexadecimal** string format.

- **Example**

        {
            "data": {
                "networkCode": "lora",
                "networkAddr": "800012ae",
                "data": "74657374"
            }
        }

- **204 No content**
- **500, 503**: See [Notes](#notes).
