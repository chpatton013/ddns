#![feature(trait_alias)]

#[macro_use]
extern crate derive_new;

extern crate bytes;
extern crate clap;
extern crate envsubst;
extern crate http;
extern crate hyper;
extern crate hyper_tls;
extern crate log;
extern crate pretty_env_logger;
extern crate serde_json;
extern crate tokio;

extern crate ddns_common;

use tokio::prelude::{future, stream, Future, Stream};

#[derive(Default, Debug, new)]
struct Config {
    update_interval: String,
    service_address: String,
    initial_address: String,
    registrar_request: String,
    update_interval_secs: u64,
    registrar_request_template: String,
}

enum ConfigError {
    ArgumentError(String),
    ParseError(String, std::num::ParseIntError),
    ReadError(String, std::io::Error),
}

impl std::fmt::Debug for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::ArgumentError(argument) => write!(
                f,
                "ConfigError(ArgumentError(Missing required argument '{}'))",
                argument
            ),
            ConfigError::ParseError(argument, inner_error) => write!(
                f,
                "ConfigError(ParseError(Failed to parse argument '{}': {:?}))",
                argument, inner_error,
            ),
            ConfigError::ReadError(argument, inner_error) => write!(
                f,
                "ConfigError(ReadError(Failed to read file from argument '{}': {:?}))",
                argument, inner_error,
            ),
        }
    }
}

type ServiceResponse = ddns_common::AddressResponse;

#[derive(Debug, new)]
struct RegistrarRequest {
    name: String,
    method: String,
    address: String,
    headers: Vec<(String, String)>,
    body: hyper::Body,
}

fn make_registrar_request(request_json: &serde_json::Value) -> RequestResult<RegistrarRequest> {
    let headers = request_json["headers"]
        .as_object()
        .ok_or(RequestError::FormatError)?;
    let headers_vec = headers
        .into_iter()
        .map(|(key, value)| (key.to_owned(), value.as_str().unwrap().to_owned()))
        .collect();

    Ok(RegistrarRequest {
        name: request_json["name"]
            .as_str()
            .ok_or(RequestError::FormatError)
            .map(|value| value.to_owned())?,
        method: request_json["method"]
            .as_str()
            .ok_or(RequestError::FormatError)
            .map(|value| value.to_owned())?,
        address: request_json["address"]
            .as_str()
            .ok_or(RequestError::FormatError)
            .map(|value| value.to_owned())?,
        headers: headers_vec,
        body: request_json["body"].to_string().into(),
    })
}

#[derive(Debug, new)]
struct TemplateError {
    template: String,
    variables: std::collections::HashMap<String, String>,
}

#[derive(Debug)]
enum RequestError {
    TemplateError(TemplateError),
    SerdeJsonError(serde_json::Error),
    FormatError,
    HttpError(http::Error),
    HyperError(hyper::Error),
    HyperTlsError(hyper_tls::Error),
}

type RequestResult<T> = Result<T, RequestError>;

#[derive(Debug, new)]
struct StatusError {
    status: hyper::StatusCode,
    body: hyper::Body,
}

#[derive(Debug)]
enum ResponseError {
    HyperError(hyper::Error),
    SerdeJsonError(serde_json::Error),
    StatusError(StatusError),
}

#[derive(Debug)]
enum DdnsError {
    IntervalError(tokio::timer::Error),
    RequestError(RequestError),
    ResponseError(ResponseError),
}

trait DdnsStream<T> = Stream<Item = T, Error = DdnsError>;
trait DdnsFuture<T> = Future<Item = T, Error = DdnsError>;
type DdnsResult<T> = Result<T, DdnsError>;

enum RequestScheme {
    Http,
    Https,
}

fn get_request_scheme(scheme_option: Option<&http::uri::Scheme>) -> RequestScheme {
    scheme_option.map_or(RequestScheme::Http, |scheme| {
        if scheme.as_str() == "https" {
            RequestScheme::Https
        } else {
            RequestScheme::Http
        }
    })
}

fn get_args() -> clap::ArgMatches<'static> {
    log::trace!("fn get_args()");

    clap::App::new("ddns_client")
        .version("0.1.0")
        .author("Christopher Patton <chpatton013@gmail.com>")
        .about("")
        .arg(
            clap::Arg::with_name("update_interval")
                .long("update_interval")
                .env("DDNS_CLIENT__UPDATE_INTERVAL")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("60")
                .help("Time interval (in seconds) between requests to the DDNS service"),
        )
        .arg(
            clap::Arg::with_name("service_address")
                .long("service_address")
                .env("DDNS_CLIENT__SERVICE_ADDRESS")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("http//0.0.0.0:3000")
                .help("URL of DDNS service"),
        )
        .arg(
            clap::Arg::with_name("initial_address")
                .long("initial_address")
                .env("DDNS_CLIENT__INITIAL_ADDRESS")
                .case_insensitive(true)
                .takes_value(true)
                .default_value("")
                .help("Current IP address registered with registrar"),
        )
        .arg(
            clap::Arg::with_name("registrar_request")
                .long("registrar_request")
                .env("DDNS_CLIENT__REGISTRAR_REQUEST")
                .case_insensitive(true)
                .takes_value(true)
                .help("Filepath of registrar request template"),
        )
        .get_matches()
}

fn make_config_from_args() -> Result<Config, Vec<ConfigError>> {
    log::trace!("fn make_config_from_args()");

    let args = get_args();

    let mut config = Config::default();
    let mut errors = Vec::default();

    match args.value_of("update_interval") {
        Some(value) => config.update_interval = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("update_interval".to_owned())),
    }
    match args.value_of("service_address") {
        Some(value) => config.service_address = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("service_address".to_owned())),
    }
    match args.value_of("initial_address") {
        Some(value) => config.initial_address = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("initial_address".to_owned())),
    }
    match args.value_of("registrar_request") {
        Some(value) => config.registrar_request = value.to_owned(),
        None => errors.push(ConfigError::ArgumentError("registrar_request".to_owned())),
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    match config.update_interval.parse::<u64>() {
        Ok(value) => config.update_interval_secs = value,
        Err(error) => errors.push(ConfigError::ParseError("update_interval".to_owned(), error)),
    }
    match std::fs::read_to_string(config.registrar_request.as_str()) {
        Ok(value) => config.registrar_request_template = value,
        Err(error) => errors.push(ConfigError::ReadError(
            "registrar_request".to_owned(),
            error,
        )),
    }

    if errors.is_empty() {
        Ok(config)
    } else {
        Err(errors)
    }
}

fn render_and_make_registrar_requests(
    registrar_request_template: &str,
    ip_address: String,
) -> DdnsResult<()> {
    log::trace!(
        "fn render_and_make_registrar_requests(registrar_request_template={:?}, ip_address={:?})",
        registrar_request_template,
        ip_address.as_str(),
    );
    render_registrar_requests(registrar_request_template, ip_address)
        .map_err(|error| DdnsError::RequestError(error))
        .and_then(|rendered_registrar_requests| {
            make_registrar_requests(rendered_registrar_requests.as_str())
                .map_err(|error| DdnsError::RequestError(error))
        })
        .map(make_registrar_futures)
}

fn render_registrar_requests(request_template: &str, ip_address: String) -> RequestResult<String> {
    log::trace!(
        "fn render_registrar_requests(request_template={:?}, ip_address={:?})",
        request_template,
        ip_address.as_str(),
    );

    let mut template_variables = std::collections::HashMap::new();
    template_variables.insert("ip_address".to_owned(), ip_address);

    envsubst::substitute(request_template, &template_variables).map_err(|_| {
        RequestError::TemplateError(TemplateError::new(
            request_template.to_owned(),
            template_variables,
        ))
    })
}

// TODO: document what this JSON format is supposed to be in a README
fn make_registrar_requests(requests_str: &str) -> RequestResult<Vec<RegistrarRequest>> {
    log::trace!(
        "fn make_registrar_requests(requests_str={:?})",
        requests_str,
    );

    let requests_result = serde_json::from_str::<serde_json::Value>(requests_str)
        .map_err(|error| RequestError::SerdeJsonError(error))?;
    let requests = requests_result
        .as_array()
        .ok_or(RequestError::FormatError)
        .map(|requests_array| {
            requests_array
                .iter()
                .map(|request_json| make_registrar_request(request_json))
                .fold(
                    (Vec::new(), Vec::new()),
                    |(mut values, mut errors), result| {
                        match result {
                            Ok(value) => values.push(value),
                            Err(error) => errors.push(error),
                        }
                        (values, errors)
                    },
                )
        });
    match requests {
        Ok((values, mut errors)) => {
            // Only report the first error. RequestResult only allows a singular
            // error to be returned. We could work around this by making a new
            // plural error type, but there's not much benefit for that amount
            // of boilerplate.
            if let Some(error) = errors.pop() {
                Err(error)
            } else {
                Ok(values)
            }
        }
        Err(error) => Err(error),
    }
}

fn make_interval_timer_stream(update_interval: u64) -> impl DdnsStream<()> {
    log::trace!(
        "fn make_interval_timer_stream(update_interval={:?})",
        update_interval
    );

    tokio::timer::Interval::new(
        std::time::Instant::now(),
        std::time::Duration::from_secs(update_interval),
    )
    .map_err(|error| DdnsError::IntervalError(error))
    .map(|_| ())
}

fn make_request_future(
    address: &str,
    method: &str,
    headers: &[(String, String)],
    body: hyper::Body,
) -> impl DdnsFuture<hyper::Response<hyper::Body>> {
    log::trace!(
        "fn make_request_future(address={:?}, method={:?}, headers={:?}, body={:?})",
        address,
        method,
        headers,
        &body,
    );

    future::result(
        make_request(address, method, headers, body)
            .map_err(|error| RequestError::HttpError(error)),
    )
    .and_then(
        |request| match get_request_scheme(request.uri().scheme_part()) {
            RequestScheme::Http => {
                let connector = hyper::client::HttpConnector::new(4);
                future::Either::A(
                    hyper::Client::builder()
                        .build::<_, hyper::Body>(connector)
                        .request(request)
                        .map_err(|error| RequestError::HyperError(error)),
                )
            }
            RequestScheme::Https => future::Either::B(
                future::result(
                    hyper_tls::HttpsConnector::new(4)
                        .map_err(|error| RequestError::HyperTlsError(error)),
                )
                .and_then(|connector| {
                    hyper::Client::builder()
                        .build::<_, hyper::Body>(connector)
                        .request(request)
                        .map_err(|error| RequestError::HyperError(error))
                }),
            ),
        },
    )
    .map_err(|error| DdnsError::RequestError(error))
}

fn make_request(
    address: &str,
    method: &str,
    headers: &[(String, String)],
    body: hyper::Body,
) -> http::Result<hyper::Request<hyper::Body>> {
    let mut builder = hyper::Request::builder();
    builder.uri(address).method(method);
    headers.into_iter().for_each(|(key, value)| {
        builder.header(key.as_str(), value.as_str());
    });
    builder.body(body)
}

fn make_service_future(address: &str) -> impl DdnsFuture<ServiceResponse> {
    log::trace!("fn make_service_future(address={:?})", address);

    log::debug!("Retrieving current IP address");

    make_service_request_future(address).and_then(decode_service_response)
}

fn make_service_request_future(address: &str) -> impl DdnsFuture<hyper::Response<hyper::Body>> {
    make_request_future(
        address,
        "GET",
        &[("Accept".to_owned(), "application/json".to_owned())],
        hyper::Body::empty(),
    )
}

fn decode_service_response(
    response: hyper::Response<hyper::Body>,
) -> impl DdnsFuture<ServiceResponse> {
    log::trace!("fn decode_service_response(response={:?})", response);

    response
        .into_body()
        .fold(
            bytes::Bytes::new(),
            |mut accumulator, chunk| -> hyper::Result<bytes::Bytes> {
                accumulator.extend_from_slice(chunk.into_bytes().as_ref());
                Ok(accumulator)
            },
        )
        .map_err(|error| DdnsError::ResponseError(ResponseError::HyperError(error)))
        .and_then(|response_bytes| {
            serde_json::from_slice(response_bytes.as_ref())
                .map_err(|error| DdnsError::ResponseError(ResponseError::SerdeJsonError(error)))
        })
}

fn make_registrar_futures(registrar_requests: Vec<RegistrarRequest>) {
    log::trace!(
        "fn make_registrar_futures(registrar_requests={:?})",
        registrar_requests,
    );

    registrar_requests.into_iter().for_each(|request| {
        tokio::spawn(make_registrar_future(request).map_err(|error| log::error!("{:?}", error)));
    });
}

fn make_registrar_future(request: RegistrarRequest) -> impl DdnsFuture<()> {
    log::trace!("fn make_registrar_future(request={:?})", &request);

    log::debug!("Updating registrar record '{}'...", request.name);

    let name = request.name.to_owned();

    make_request_future(
        request.address.as_str(),
        request.method.as_str(),
        request.headers.as_slice(),
        request.body,
    )
    .and_then(move |response| process_registrar_response(name.as_str(), response))
}

fn process_registrar_response(
    name: &str,
    response: hyper::Response<hyper::Body>,
) -> impl DdnsFuture<()> {
    log::trace!(
        "fn process_registrar_response(name={:?}, response={:?})",
        name,
        response
    );

    let status_code = response.status().as_u16();
    if status_code < 200 || status_code >= 300 {
        log::warn!(
            "Failed to update registrar record '{}': {:?}",
            name,
            response
        );
        future::err(DdnsError::ResponseError(ResponseError::StatusError(
            StatusError::new(response.status(), response.into_body()),
        )))
    } else {
        log::debug!("Successfully updated registrar record '{}'", name);
        future::ok(())
    }
}

fn main() {
    pretty_env_logger::init();

    let config = make_config_from_args()
        .map_err(|error| log::error!("{:?}", error))
        .unwrap();
    log::info!("Initialized with {:?}", config);

    let update_interval_secs = config.update_interval_secs;
    let service_address = config.service_address;
    let registrar_request_template = config.registrar_request_template;

    let mut ip_address: Option<String> = None;
    if !config.initial_address.is_empty() {
        ip_address.replace(config.initial_address.clone());
    }

    tokio::run(
        make_interval_timer_stream(update_interval_secs)
            .and_then(move |_| make_service_future(service_address.as_str()))
            .and_then(move |service_response| {
                log::trace!("closure process_service_response({:?})", service_response);

                if ip_address.as_ref() == Some(&service_response.ip) {
                    log::debug!("IP Address unchanged from {:?}", ip_address);
                    return future::ok(());
                }

                log::info!(
                    "IP Address has changed from {:?} to {}",
                    ip_address,
                    service_response.ip,
                );
                ip_address.replace(service_response.ip.clone());

                future::result(render_and_make_registrar_requests(
                    registrar_request_template.as_str(),
                    service_response.ip,
                ))
            })
            .map_err(|error| log::error!("{:?}", error))
            .then(|r| future::ok(stream::iter_ok::<_, ()>(r)))
            .for_each(|_| Ok(())),
    );
}
