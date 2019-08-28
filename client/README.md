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
