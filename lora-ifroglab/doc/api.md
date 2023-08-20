API - lora-ifroglab
===================

## Contents

- [Data APIs](#data)
    - [`GET /lora-ifroglab/api/v1/data/uldata` Get latest uplink data](#get_data_uldata)
    - [`GET /lora-ifroglab/api/v1/data/dldata` Get latest downlink data](#get_data_dldata)
    - [`GET /lora-ifroglab/api/v1/data/queue/{networkAddr}` Get queuing downlink data](#get_data_queue)

## <a name="notes"></a>Notes

All API requests (except `GET /version`) must have a **Authorization** header with a **Bearer** token.

- **Example**

    ```http
    GET /auth/api/v1/user HTTP/1.1
    Host: localhost
    Authorization: Bearer 766f29fa8691c81b749c0f316a7af4b7d303e45bf4000fe5829365d37caec2a4
    ```

All APIs may respond one of the following status codes:

- **200 OK**: The request is success with body.
- **204 No Content**: The request is success without body.
- **400 Bad Request**: The API request has something wrong.
- **401 Unauthorized**: The access token is invalid or expired.
- **403 Forbidden**: The user does not have the permission to operate APIs.
- **404 Not Found**: The resource (in path) does not exist.
- **500 Internal Server Error**: The server is crash or get an unknown error. You should respond to the system administrators to solve the problem.
- **503 Service Unavailable**: The server has something wrong. Please try again later.

All error responses have the following parameters in JSON format string:

- *string* `code`: The error code.
- *string* `message`: (**optional**) The error message.

- **Example**

    ```http
    HTTP/1.1 401 Unauthorized
    Access-Control-Allow-Origin: *
    Content-Type: application/json
    Content-Length: 70
    ETag: W/"43-Npr+dy47IJFtraEIw6D8mYLw7Ws"
    Date: Thu, 13 Jan 2022 07:46:09 GMT
    Connection: keep-alive

    {"code":"err_auth","message":"Invalid token: access token is invalid"}

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
