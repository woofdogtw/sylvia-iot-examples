API - lora-ifroglab
===================

## Contents

- [Data APIs](#data)
    - [`GET /lora-ifroglab/api/v1/data/uldata` Get latest uplink data](#get_data_uldata)
    - [`GET /lora-ifroglab/api/v1/data/dldata` Get latest downlink data](#get_data_dldata)
    - [`GET /lora-ifroglab/api/v1/data/queue/{networkAddr}` Get queuing downlink data](#get_data_queue)

# <a name="data"></a>Data APIs

## <a name="get_data_uldata"></a>Get latest uplink data

Get latest 100 uplink data from all nodes.

    GET /lora-ifroglab/api/v1/data/uldata

#### Response

- **200 OK**: Latest uplink data. Parameters are:

    - *object[]* `data`:
        - *string* `time`: Device time for this data in ISO 8601 format.
        - *string* `networkAddr`: Node address.
        - *string* `data`: Payload data in hexadecimal string.
        - *object* `extension`: Extension data.
            - *number* `rssi`: The RSSI value of the data.

- **500, 503**: See [Notes](#notes).

## <a name="get_data_dldata"></a>Get latest downlink data

Get latest 100 downlink data from the application server.

    GET /lora-ifroglab/api/v1/data/dldata

#### Response

- **200 OK**: Latest downlink data. Parameters are:

    - *object[]* `data`:
        - *string* `time`: The received time from the queue in ISO 8601 format.
        - *string* `pub`: The published time from the broker in ISO 8601 format.
        - *string* `sent`: The sent time when sending the **0x05** command.
        - *string* `networkAddr`: Node address.
        - *string* `data`: Payload data in hexadecimal string.

- **500, 503**: See [Notes](#notes).

## <a name="get_data_queue"></a>Get queuing downlink data

Get queuing downlink data from the application server.

    GET /lora-ifroglab/api/v1/data/queue/{networkAddr}

- *string* `networkAddr`: The specified network address.

#### Response

- **200 OK**: Latest downlink data. Parameters are:

    - *object[]* `data`:
        - *string* `time`: The received time from the queue in ISO 8601 format.
        - *string* `pub`: The published time from the broker in ISO 8601 format.
        - *string* `networkAddr`: Node address.
        - *string* `data`: Payload data in hexadecimal string.

- **500, 503**: See [Notes](#notes).
