# ddns_external_server

A dynamic DNS server that can be run on any device outside of your internal
network. This server is meant for network configurations where you cannot run
software on your gateway device. The external server responds to all requests
with the IP address of the remote host.

## Usage

```
ddns_external_server --host=0.0.0.0 --port=3000
```

## Configuration

The external server must be configured with a host and port to listen on. These
can be provided via command-line arguments or environment variables.

### Host

Host address this server should listen on.

Command-line argument: `--host`

Environment variable: `DDNS_EXTERNAL_SERVER__HOST`

Default value: `0.0.0.0`

### Port

Host port this server should listen on.

Command-line argument: `--port`

Environment variable: `DDNS_EXTERNAL_SERVER__PORT`

Default value: `3000`
