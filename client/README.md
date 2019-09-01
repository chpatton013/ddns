# ddns_client

The dynamic DNS client must be run on a device that can communicate with the
device running the dynamic DNS server and your domain name registrar. The client
makes periodic requests to the server, checking for an updated IP address. When
an IP address update is detected, the client makes requests against your
registrar to update any DNS records you have configured.

## Usage

```
ddns_client \
  --update_interval=60 \
  --service_address=http//0.0.0.0:3000 \
  --initial_address=10.0.0.1 \
  --registrar_request=./request.json
```

## Configuration

The client must be configured with the network interface to monitor and
the host and port to listen on. These can be provided via command-line arguments
or environment variables.

### Update Interval

Time interval (in seconds) between requests to the DDNS service.

Command-line argument: `--update_interval`

Environment variable: `DDNS_CLIENT__UPDATE_INTERVAL`

Default value: `60`

### Service Address

URL of DDNS service.

Command-line argument: `--service_address`

Environment variable: `DDNS_CLIENT__SERVICE_ADDRESS`

Default value: `http//0.0.0.0:3000`

### Initial Address

Current IP address registered with registrar.

Command-line argument: `--initial_address`

Environment variable: `DDNS_CLIENT__INITIAL_ADDRESS`

Default value: `(none)`

### Registrar Request

Filepath of registrar request template.

Command-line argument: `--registrar_request`

Environment variable: `DDNS_CLIENT__REGISTRAR_REQUEST`

## Request file format

The request file should contain a JSON-encoded list of request templates to
perform when the client detects a chance in the tracked IP address.

Each element in this request list should be an object with the following
properties:
* `name`: (string) A unique name to identify this request instance. This is used
  only internally to the client application for logging purposes.
* `method`: (string) The HTTP request method for this request.
* `address`: (string) The URI of the registrar to make the request against.
* `headers`: (object[string:string]): An object of request headers to include in
  the registrar request. Keys are the header name, and values are the header
  content.
* `body`: The body of the registrar request. This can be any JSON-type, but a
  string or object is most common. Regardless of the type, this field will be
  coerced to a string on import.

Note that credentials in the `Authorization` header must be base64-encoded.
For example, instead of:
```
"Authorization": "Basic username:password"
```
You should supply:
```
"Authorization": "Basic dXNlcm5hbWU6cGFzc3dvcmQK"
```

Note that the `body` field is a treated as a template string. When a new IP
address is detected by the client, this field will be treated as an
envsubst-compatible template, and the substring `${ip_address}` will be replaced
with the string value of the new IP address.
