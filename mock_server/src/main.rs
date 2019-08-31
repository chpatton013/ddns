#[macro_use]
extern crate derive_new;

extern crate clap;
extern crate hyper;
extern crate log;
extern crate pretty_env_logger;

#[derive(Debug, new)]
struct Config {
    host: String,
    port: String,
    status: String,
    headers: Vec<String>,
    body: String,
    socket_address: std::net::SocketAddr,
    response_status: u16,
    response_headers: Vec<(String, String)>,
}

enum ConfigError {
    ArgumentError(String),
    SocketAddressParseError(String, String, std::net::AddrParseError),
    StatusCodeParseError(String, std::num::ParseIntError),
}

impl std::fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ArgumentError(argument) => write!(
                f,
                "ConfigError(ArgumentError(Missing required argument '{}'))",
                argument
            ),
            ConfigError::SocketAddressParseError(host, port, inner_error) => write!(
          f,
          "ConfigError(SocketAddressParseError(Failed to parse socket address '{}:{}': {:?}))",
          host, port, inner_error,
          ),
            ConfigError::StatusCodeParseError(status, inner_error) => write!(
                f,
                "ConfigError(StatusCodeParseError(Failed to parse status code '{}': {:?}))",
                status, inner_error,
            ),
        }
    }
}

fn get_args() -> clap::ArgMatches<'static> {
    log::trace!("fn get_args()");

    clap::App::new("ddns_mock_server")
        .version("0.1.0")
        .author("Christopher Patton <chpatton013@gmail.com>")
        .about("Responds to requests with the configured response")
        .arg(
            clap::Arg::with_name("host")
                .long("host")
                .env("DDNS_MOCK_SERVER__HOST")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("0.0.0.0")
                .help("Host address this server should listen on"),
        )
        .arg(
            clap::Arg::with_name("port")
                .long("port")
                .env("DDNS_MOCK_SERVER__PORT")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("3000")
                .help("Host port this server should listen on"),
        )
        .arg(
            clap::Arg::with_name("status")
                .long("status")
                .env("DDNS_MOCK_SERVER__STATUS")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("200")
                .help("Status code this server should respond with"),
        )
        .arg(
            clap::Arg::with_name("header")
                .long("header")
                .env("DDNS_MOCK_SERVER__HEADER")
                .case_insensitive(true)
                .multiple(true)
                .takes_value(true)
                .default_value("Content-Type application/json")
                .help("Response headers this server should respond with"),
        )
        .arg(
            clap::Arg::with_name("body")
                .long("body")
                .env("DDNS_MOCK_SERVER__BODY")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("{\"ip\":\"0.0.0.0\"}")
                .help("Response body this server should respond with"),
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

    let mut status = String::default();
    match args.value_of("status") {
        Some(value) => status = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("status".to_owned())),
    }

    let mut headers = vec![];
    match args.values_of("header") {
        Some(values) => headers = values.map(|header| header.to_owned()).collect(),
        None => errors.push(ConfigError::ArgumentError("header".to_owned())),
    }

    let mut body = String::default();
    match args.value_of("body") {
        Some(value) => body = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("body".to_owned())),
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let mut socket_address_option = None;
    match format!("{}:{}", host.as_str(), port.as_str()).parse() {
        Ok(socket_address) => {
            socket_address_option.replace(socket_address);
        }
        Err(error) => errors.push(ConfigError::SocketAddressParseError(
            host.clone(),
            port.clone(),
            error,
        )),
    }
    let socket_address = socket_address_option.unwrap();

    let mut response_status = u16::default();
    match status.parse::<u16>() {
        Ok(value) => response_status = value,
        Err(error) => errors.push(ConfigError::StatusCodeParseError(status.clone(), error)),
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let response_headers = headers.iter().fold(vec![], |mut accumulator, header| {
        accumulator.push((
            header.split_whitespace().take(1).collect(),
            header.split_whitespace().skip(1).collect(),
        ));
        accumulator
    });

    Ok(Config::new(
        host,
        port,
        status,
        headers,
        body,
        socket_address,
        response_status,
        response_headers,
    ))
}

fn respond(
    request: hyper::Request<hyper::Body>,
    response_status: &u16,
    response_headers: &[(String, String)],
    response_body: &str,
) -> hyper::Response<hyper::Body> {
    log::trace!(
        "fn respond(request={:?}, response_status={:?}, response_headers={:?}, response_body={:?})",
        &request,
        response_status,
        response_headers,
        response_body,
    );

    log::debug!("{:?}", request);

    let mut builder = hyper::Response::builder();
    builder.status(response_status.to_owned());
    response_headers.iter().for_each(|(key, value)| {
        builder.header(key, value);
    });
    let response = builder
        .body(hyper::Body::from(response_body.to_owned()))
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
    let response_status = config.response_status;
    let response_headers = config.response_headers;
    let response_body = config.body;

    let new_service = move || {
        let response_status_clone = response_status.clone();
        let response_headers_clone = response_headers.clone();
        let response_body_clone = response_body.clone();
        hyper::service::service_fn_ok(move |request| {
            respond(
                request,
                &response_status_clone,
                response_headers_clone.as_slice(),
                response_body_clone.as_str(),
            )
        })
    };

    let server = hyper::Server::bind(&socket_address).serve(new_service);
    log::info!("Listening on http://{}", socket_address);

    use hyper::rt::Future;
    hyper::rt::run(server.map_err(|error| {
        log::error!("{:?}", error);
    }));
}
