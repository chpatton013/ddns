# ddns_gateway_server

A dynamic DNS server that must be run on your gateway address. This server is
meant for network configurations where you can run software on your gateway
device. The gateway server responds to all requests with the IP address of the
configured network interface.

## Usage

```
ddns_gateway_server --interface=eth0 --host=0.0.0.0 --port=3000
```

## Configuration

The gateway server must be configured with the network interface to monitor and
the host and port to listen on. These can be provided via command-line arguments
or environment variables.

### Interface

Network interface to report IP address from.

Command-line argument: `--interface`

Environment variable: `DDNS_GATEWAY_SERVER__INTERFACE`

### Host

Host address this server should listen on.

Command-line argument: `--host`

Environment variable: `DDNS_GATEWAY_SERVER__HOST`

Default value: `0.0.0.0`

### Port

Host port this server should listen on.

Command-line argument: `--port`

Environment variable: `DDNS_GATEWAY_SERVER__PORT`

Default value: `3000`
