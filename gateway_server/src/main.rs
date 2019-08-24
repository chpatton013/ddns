#[macro_use]
extern crate derive_new;

extern crate clap;
extern crate hyper;
extern crate log;
extern crate nix;
extern crate pretty_env_logger;
extern crate serde_json;

extern crate ddns_common;

#[derive(Debug, new)]
struct Config {
    interface: String,
    host: String,
    port: String,
    socket_address: std::net::SocketAddr,
}

enum ConfigError {
    ArgumentError(String),
    ParseError(String, String, std::net::AddrParseError),
}

impl std::fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ArgumentError(argument) => write!(
                f,
                "ConfigError(ArgumentError(Missing required argument '{}'))",
                argument
            ),
            ConfigError::ParseError(host, port, inner_error) => write!(
                f,
                "ConfigError(ParseError(Failed to parse socket address '{}:{}': {:?}))",
                host, port, inner_error,
            ),
        }
    }
}

use ddns_common::AddressResponse;

#[derive(serde::Serialize)]
struct NoAddressesResponse {
    interface: String,
    message: String,
}

#[derive(serde::Serialize)]
struct MultipleAddressesResponse {
    interface: String,
    addresses: Vec<String>,
    message: String,
}

enum Response {
    Address(AddressResponse),
    NoAddresses(NoAddressesResponse),
    MultipleAddresses(MultipleAddressesResponse),
}

impl Response {
    fn to_json(&self) -> serde_json::Result<String> {
        match self {
            Response::Address(response) => serde_json::to_string(response),
            Response::NoAddresses(response) => serde_json::to_string(response),
            Response::MultipleAddresses(response) => serde_json::to_string(response),
        }
    }
}

fn get_args() -> clap::ArgMatches<'static> {
    log::trace!("fn get_args()");

    clap::App::new("ddns_gateway")
        .version("0.1.0")
        .author("Christopher Patton <chpatton013@gmail.com>")
        .about("Responds to requests with the IP address of a network interface")
        .arg(
            clap::Arg::with_name("interface")
                .long("interface")
                .env("DDNS_GATEWAY_SERVER__INTERFACE")
                .case_insensitive(true)
                .takes_value(true)
                .help("Network interface to report IP address from"),
        )
        .arg(
            clap::Arg::with_name("host")
                .long("host")
                .env("DDNS_GATEWAY_SERVER__HOST")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("0.0.0.0")
                .help("Host address this server should listen on"),
        )
        .arg(
            clap::Arg::with_name("port")
                .long("port")
                .env("DDNS_GATEWAY_SERVER__PORT")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("3000")
                .help("Host port this server should listen on"),
        )
        .get_matches()
}

fn make_config_from_args() -> Result<Config, Vec<ConfigError>> {
    log::trace!("fn make_config_from_args()");

    let args = get_args();

    let mut errors = Vec::default();

    let mut interface = String::default();
    match args.value_of("interface") {
        Some(value) => interface = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("interface".to_owned())),
    }

    let mut host = String::default();
    match args.value_of("host") {
        Some(value) => host = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("host".to_owned())),
    }

    let mut port = String::default();
    match args.value_of("port") {
        Some(value) => port = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("port".to_owned())),
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    match format!("{}:{}", host.as_str(), port.as_str()).parse() {
        Ok(socket_address) => Ok(Config::new(interface, host, port, socket_address)),
        Err(error) => Err(vec![ConfigError::ParseError(host, port, error)]),
    }
}

fn is_matching_inet_interface_address(
    interface_address: &nix::ifaddrs::InterfaceAddress,
    interface: &str,
) -> bool {
    interface_address.interface_name == interface
        && interface_address.address.is_some()
        && interface_address.netmask.is_some()
        && interface_address.broadcast.is_some()
}

fn matching_inet_ip_addrs(interface: &str) -> Vec<nix::sys::socket::IpAddr> {
    log::trace!("fn matching_inet_ip_addrs(interface={:?})", interface,);

    nix::ifaddrs::getifaddrs()
        .unwrap()
        .filter_map(|interface_address| {
            if !is_matching_inet_interface_address(&interface_address, interface) {
                return None;
            }

            let sock_addr = interface_address.address.unwrap();
            match sock_addr {
                nix::sys::socket::SockAddr::Inet(inet_addr) => Some(inet_addr.ip()),
                _ => None,
            }
        })
        .collect()
}

fn make_response(interface: &str, ip_addresses: &[nix::sys::socket::IpAddr]) -> (u16, Response) {
    log::trace!(
        "fn make_response(interface={:?}, ip_addresses={:?})",
        interface,
        ip_addresses,
    );

    if ip_addresses.len() == 0 {
        (
            500,
            Response::NoAddresses(NoAddressesResponse {
                interface: interface.to_string(),
                message: format!("No inet addresses found for interface {}", interface),
            }),
        )
    } else if ip_addresses.len() > 1 {
        let ip_address_strings = ip_addresses.iter().map(|ip_addr| format!("{}", ip_addr));
        (
            500,
            Response::MultipleAddresses(MultipleAddressesResponse {
                interface: interface.to_string(),
                message: format!(
                    "Multiple inet addresses found for interface {}: {}",
                    interface,
                    ip_address_strings
                        .to_owned()
                        .fold(String::new(), |accumulator, ip_addr| {
                            accumulator + ip_addr.as_str()
                        })
                ),
                addresses: ip_address_strings.collect(),
            }),
        )
    } else {
        (
            200,
            Response::Address(AddressResponse {
                ip: format!("{}", ip_addresses.iter().next().unwrap()),
            }),
        )
    }
}

fn respond(request: hyper::Request<hyper::Body>, interface: &str) -> hyper::Response<hyper::Body> {
    log::trace!(
        "fn respond(request={:?}, interface={:?})",
        &request,
        interface,
    );

    let ip_addresses = matching_inet_ip_addrs(interface);
    let (status, response) = make_response(interface, ip_addresses.as_slice());
    let response = hyper::Response::builder()
        .header("Content-Type", "application/json")
        .status(status)
        .body(hyper::Body::from(response.to_json().unwrap()))
        .unwrap();

    log::debug!("{:?}", response);
    response
}

fn main() {
    pretty_env_logger::init();

    let config = make_config_from_args()
        .map_err(|error| log::error!("{:?}", error))
        .unwrap();
    log::info!("Initialized with {:?}", config);

    let interface = config.interface;
    let socket_address = config.socket_address;

    let new_service = move || {
        let interface_clone = interface.to_owned();
        hyper::service::service_fn_ok(move |request| respond(request, interface_clone.as_str()))
    };

    let server = hyper::Server::bind(&socket_address).serve(new_service);
    log::info!("Listening on http://{}", socket_address);

    use hyper::rt::Future;
    hyper::rt::run(server.map_err(|error| {
        log::error!("{:?}", error);
    }));
}
