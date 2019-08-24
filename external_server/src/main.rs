#[macro_use]
extern crate derive_new;

extern crate clap;
extern crate hyper;
extern crate log;
extern crate pretty_env_logger;
extern crate serde_json;

extern crate ddns_common;

#[derive(Debug, new)]
struct Config {
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

fn get_args() -> clap::ArgMatches<'static> {
    log::trace!("fn get_args()");

    clap::App::new("ddns_external_server")
        .version("0.1.0")
        .author("Christopher Patton <chpatton013@gmail.com>")
        .about("Responds to requests with the remote IP address")
        .arg(
            clap::Arg::with_name("host")
                .long("host")
                .env("DDNS_EXTERNAL_SERVER__HOST")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("0.0.0.0")
                .help("Host address this server should listen on"),
        )
        .arg(
            clap::Arg::with_name("port")
                .long("port")
                .env("DDNS_EXTERNAL_SERVER__PORT")
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
        Ok(socket_address) => Ok(Config::new(host, port, socket_address)),
        Err(error) => Err(vec![ConfigError::ParseError(host, port, error)]),
    }
}

fn respond(
    request: hyper::Request<hyper::Body>,
    remote_addr: std::net::SocketAddr,
) -> hyper::Response<hyper::Body> {
    println!("{:?}", request);
    log::trace!(
        "fn respond(request={:?}, remote_addr={:?})",
        &request,
        &remote_addr,
    );

    let address_response = AddressResponse {
        ip: remote_addr.ip().to_string(),
    };
    let body_json = serde_json::to_string(&address_response).unwrap();
    let response = hyper::Response::builder()
        .header("Content-Type", "application/json")
        .body(hyper::Body::from(body_json))
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

    let socket_address = config.socket_address;

    let new_service =
        hyper::service::make_service_fn(|socket: &hyper::server::conn::AddrStream| {
            let remote_addr = socket.remote_addr();
            hyper::service::service_fn_ok(move |request| respond(request, remote_addr))
        });

    let server = hyper::Server::bind(&socket_address).serve(new_service);
    log::info!("Listening on http://{}", socket_address);

    use hyper::rt::Future;
    hyper::rt::run(server.map_err(|error| {
        log::error!("{:?}", error);
    }));
}
