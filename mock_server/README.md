# ddns_mock_server

A dynamic DNS server that is meant for testing the DDNS client. The mock server
responds to all requests with the values provided to it on the command-line.

## Usage

```
ddns_mock_server \
  --host=0.0.0.0 \
  --port=3000 \
  --status=200 \
  --header="Content-Type application/json" \
  --body="{\"ip\":\"0.0.0.0\"}"
```

## Configuration

The external server must be configured with a host and port to listen on. These
can be provided via command-line arguments or environment variables.

### Host

Host address this server should listen on.

Command-line argument: `--host`

Environment variable: `DDNS_MOCK_SERVER__HOST`

Default value: `0.0.0.0`

### Port

Host port this server should listen on.

Command-line argument: `--port`

Environment variable: `DDNS_MOCK_SERVER__PORT`

Default value: `3000`

### Status

Status code this server should respond with.

Command-line argument: `--status`

Environment variable: `DDNS_MOCK_SERVER__STATUS`

Default value: `200`

### Header

Response headers this server should respond with.

Command-line argument: `--header`

Environment variable: `DDNS_MOCK_SERVER__HEADER`

Default value: `Content-Type application/json`

### Body

Response body this server should respond with.

Command-line argument: `--body`

Environment variable: `DDNS_MOCK_SERVER__BODY`

Default value: `{"ip":"0.0.0.0"}`
