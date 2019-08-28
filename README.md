# ddns

*Hand-rolled dynamic DNS in Rust*

## Summary

Client and server applications to support dynamic DNS for your registrar of
choice.

## Project Organization

`ddns` is divided into three different packages: `external_server`,
`gateway_server`, and `client`. To use `ddns` you will need to run one of the
two server binaries as well as the client binary.

### `external_server`

The external server binary is intended to be run on separate hardware outside of
your private network. This is useful for network configurations where you cannot
run software on your gateway device. The external server responds to all
requests with the IP address of the remote host.

See more details in the `external_server` package
[README.md](external_server/README.md).

### `gateway_server`

The gateway server binary is intended to be run on your gateway device, which
requires that you have the ability to run arbitrary software on this device. The
gateway server responds to all requests with the IP address of one of its
network interfaces.

See more details in the `gateway_server` package
[README.md](gateway_server/README.md).

### `client`

The client binary can be run on any device that can communicate with the device
running the server and your domain name registrar. The client makes periodic
requests to the server, checking for an updated IP address. When an IP address
update is detected, the client makes requests against your registrar to update
any DNS records you have configured.

See more details in the `client` package
[README.md](client/README.md).

## License

`ddns` is licensed under the terms of the MIT License, as described in
[LICENSE.md](LICENSE.md)

## Contributing

Contributions are welcome in the form of bug reports, feature requests, or pull
requests.

Contribution to ddns is organized under the terms of the [Contributor
Covenant](CONTRIBUTOR_COVENANT.md).
