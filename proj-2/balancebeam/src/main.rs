mod internal;

mod request;
mod response;

use clap::Parser;
use crossbeam_channel::{self, Receiver, Sender};
use http::Request;
use internal::proxy_status::{ProxyState, UpstreamUnit};
use rand::{Rng, SeedableRng};
use reqwest;
use std::{
    collections::HashMap,
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
    thread::JoinHandle,
};
use std::{thread, time};

use crate::internal::proxy_status::UpstreamStatus;

/// Contains information parsed from the command-line invocation of balancebeam. The Clap macros
/// provide a fancy way to automatically construct a command-line argument parser.
#[derive(Parser, Debug)]
#[command(about = "Fun with load balancing")]
struct CmdOptions {
    // IP/port to bind to
    #[arg(short, long, default_value = "0.0.0.0:1100")]
    bind: String,
    // Upstream host to forward request to
    #[arg(short, long)]
    upstream: Vec<String>,
    // Perform active health checks on this interval (in seconds)
    #[arg(long, default_value = "10")]
    active_health_check_interval: usize,
    // "Path to send request to for active health checks"
    #[arg(long, default_value = "/")]
    active_health_check_path: String,
    //"Maximum number of requests to accept per IP per minute (0 = unlimited)"
    #[arg(long, default_value = "0")]
    max_requests_per_minute: usize,
}

fn main() {
    // Initialize the logging library. You can print log messages using the `log` macros:
    // https://docs.rs/log/0.4.8/log/ You are welcome to continue using print! statements; this
    // just looks a little prettier.
    if let Err(_) = std::env::var("RUST_LOG") {
        std::env::set_var("RUST_LOG", "debug");
    } //seems like a universal way to enable logging even in library.
    pretty_env_logger::init();

    // Parse the command line arguments passed to this program
    let options = CmdOptions::parse();
    if options.upstream.len() < 1 {
        log::error!("At least one upstream server must be specified using the --upstream option.");
        std::process::exit(1);
    }

    // Start listening for connections
    let listener = match TcpListener::bind(&options.bind) {
        Ok(listener) => listener,
        Err(err) => {
            log::error!("Could not bind to {}: {}", options.bind, err);
            std::process::exit(1);
        }
    };
    log::info!("Listening for requests on {}", options.bind);
    let mut upstream_status = HashMap::new();
    for addr in options.upstream.clone() {
        upstream_status.insert(
            addr.clone(),
            UpstreamUnit {
                address: addr,
                fail: false,
            },
        );
    }
    let mut ups = Arc::new(Mutex::new(UpstreamStatus::new(
        options.upstream.clone(),
        upstream_status,
    )));
    // Handle incoming connections
    let (success_sender, success_receiver) = crossbeam_channel::unbounded();
    let (fail_sender, fail_receiver) = crossbeam_channel::unbounded();
    let state = ProxyState::new(
        options.active_health_check_interval,
        options.active_health_check_path,
        options.max_requests_per_minute,
        ups,
        success_receiver.clone(),
        fail_receiver.clone(),
    );
    handle_active_health_check(
        &state,
        &success_sender,
        &fail_sender,
        &success_receiver,
        &fail_receiver,
    );
    // channel to accept input ,
    // create a background proxy status main thread.
    // create another thread handling proxy status.

    for stream in listener.incoming() {
        if let Ok(stream) = stream {
            // Handle the connection!
            handle_connection(stream, &state);
        }
    }
}

fn handle_active_health_check(
    state: &ProxyState,
    ss: &Sender<String>,
    fs: &Sender<String>,
    sr: &Receiver<String>,
    fr: &Receiver<String>,
) -> Option<JoinHandle<()>> {
    let options = state.get_option();
    let suc = ss.clone();
    let fail = fs.clone();
    let handle = thread::spawn(move || {
        active_check_routine(options, suc, fail);
    });
    let hc_handle = state.main_routine_invoker(sr, fr);
    Some(handle)
}

fn active_check_routine(options: CmdOptions, ss: Sender<String>, fs: Sender<String>) {
    //println!("{}", options.upstream[0]);
    loop {
        for up_addr in options.upstream.iter() {
            //make a http request to path
            let path = format!(
                "http://{}{}",
                up_addr.clone(),
                options.active_health_check_path.clone()
            );
            let mut res = match reqwest::blocking::get(&path) {
                Ok(res) => res,
                Err(_e) => {
                    log::debug!("Send request failed!");
                    //fail send to channel
                    fs.send(up_addr.to_string());
                    continue;
                }
            };
            let text = match res.text() {
                Ok(res) => res,
                Err(_e) => {
                    //fail send to channel
                    log::debug!("parse text failed!");
                    fs.send(up_addr.to_string());
                    continue;
                }
            };
            ss.send(up_addr.to_string());
            //println!("{}", text);
            //success! send to channel
        }
        thread::sleep(time::Duration::from_secs(
            options.active_health_check_interval as u64,
        ));
    }
}
fn handle_health_fail() {}
fn handle_health_success() {}

fn connect_to_upstream(state: &ProxyState) -> Result<TcpStream, std::io::Error> {
    //check the historical availablity
    loop {
        match state.select_random_updastream() {
            //how to build a long living connection?
            Some(addr) => match TcpStream::connect(addr.clone()) {
                Ok(conn) => {
                    return Ok(conn);
                }
                Err(e) => {
                    //notify this is not available.
                    state.noti_fail(&addr);
                    log::error!("Failed to connect to upstream {}: {}", addr, e);
                }
            },
            None => {
                break;
            }
        }

        // .or_else(|err| {
        //     log::error!("Failed to connect to upstream {}: {}", upstream_ip, err);
        //     success = false;
        // })
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::Other,
        "no upstream could be connected to",
    ))
    // TODO: implement failover (milestone 3)
}

fn send_response(client_conn: &mut TcpStream, response: &http::Response<Vec<u8>>) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!(
        "{} <- {}",
        client_ip,
        response::format_response_line(&response)
    );
    if let Err(error) = response::write_to_stream(&response, client_conn) {
        log::warn!("Failed to send response to client: {}", error);
        return;
    }
}

fn handle_connection(mut client_conn: TcpStream, state: &ProxyState) {
    let client_ip = client_conn.peer_addr().unwrap().ip().to_string();
    log::info!("Connection received from {}", client_ip);

    // Open a connection to a random destination server
    let mut upstream_conn = match connect_to_upstream(state) {
        Ok(stream) => stream,
        Err(_error) => {
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response);
            return;
        }
    };
    let upstream_ip = client_conn.peer_addr().unwrap().ip().to_string();

    // The client may now send us one or more requests. Keep trying to read requests until the
    // client hangs up or we get an error.
    loop {
        // PITFALL: does not solve the problem, outer connection might has been ended. SEND REQUEST TO UPSTREAM FAIL.
        // Read a request from the client
        let mut request = match request::read_from_stream(&mut client_conn) {
            Ok(request) => request,
            // Handle case where client closed connection and is no longer sending requests
            Err(request::Error::IncompleteRequest(0)) => {
                log::debug!("Client finished sending requests. Shutting down connection");
                return;
            }
            // Handle I/O error in reading from the client
            Err(request::Error::ConnectionError(io_err)) => {
                log::info!("Error reading request from client stream: {}", io_err);
                return;
            }
            Err(error) => {
                log::debug!("Error parsing request: {:?}", error);
                let response = response::make_http_error(match error {
                    request::Error::IncompleteRequest(_)
                    | request::Error::MalformedRequest(_)
                    | request::Error::InvalidContentLength
                    | request::Error::ContentLengthMismatch => http::StatusCode::BAD_REQUEST,
                    request::Error::RequestBodyTooLarge => http::StatusCode::PAYLOAD_TOO_LARGE,
                    request::Error::ConnectionError(_) => http::StatusCode::SERVICE_UNAVAILABLE,
                });
                send_response(&mut client_conn, &response);
                continue;
            }
        };
        log::info!(
            "{} -> {}: {}",
            client_ip,
            upstream_ip,
            request::format_request_line(&request)
        );

        // Add X-Forwarded-For header so that the upstream server knows the client's IP address.
        // (We're the ones connecting directly to the upstream server, so without this header, the
        // upstream server will only know our IP, not the client's.)
        request::extend_header_value(&mut request, "x-forwarded-for", &client_ip);

        // Forward the request to the server
        if let Err(error) = request::write_to_stream(&request, &mut upstream_conn) {
            log::error!(
                "Failed to send request to upstream {}: {}",
                upstream_ip,
                error
            );
            let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
            send_response(&mut client_conn, &response);
            return;
        }
        log::debug!("Forwarded request to server");

        // Read the server's response
        let response = match response::read_from_stream(&mut upstream_conn, request.method()) {
            Ok(response) => response,
            Err(error) => {
                log::error!("Error reading response from server: {:?}", error);
                let response = response::make_http_error(http::StatusCode::BAD_GATEWAY);
                send_response(&mut client_conn, &response);
                return;
            }
        };
        // Forward the response to the client
        send_response(&mut client_conn, &response);
        log::debug!("Forwarded response to client");
    }
}
