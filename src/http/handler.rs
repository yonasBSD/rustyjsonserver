use crate::config::compiled::CompiledMethodResponse;
use crate::http::router::RoutesData;
use crate::rjscript;
use crate::rjscript::evaluator::runtime::value::RJSValue;
use serde_json;
use tokio::io::{self, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tracing::error;

use super::request::{parse_http_request, Request};
use super::router::find_route;

const NOT_FOUND: &str = "HTTP/1.1 404 NOT FOUND\r\n\r\n";
const INTERNAL_SERVER_ERROR: &str = "HTTP/1.1 500 INTERNAL SERVER ERROR\r\n\r\n";
const SERVICE_UNAVAILABLE: &str = "HTTP/1.1 503 SERVICE UNAVAILABLE\r\n\r\n";

fn reason_phrase(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        101 => "Switching Protocols",
        102 => "Processing",
        103 => "Early Hints",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        203 => "Non-Authoritative Information",
        204 => "No Content",
        205 => "Reset Content",
        206 => "Partial Content",
        207 => "Multi-Status",
        208 => "Already Reported",
        226 => "IM Used",
        300 => "Multiple Choices",
        301 => "Moved Permanently",
        302 => "Found",
        303 => "See Other",
        304 => "Not Modified",
        305 => "Use Proxy",
        307 => "Temporary Redirect",
        308 => "Permanent Redirect",
        400 => "Bad Request",
        401 => "Unauthorized",
        402 => "Payment Required",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        407 => "Proxy Authentication Required",
        408 => "Request Timeout",
        409 => "Conflict",
        410 => "Gone",
        411 => "Length Required",
        412 => "Precondition Failed",
        413 => "Payload Too Large",
        414 => "URI Too Long",
        415 => "Unsupported Media Type",
        416 => "Range Not Satisfiable",
        417 => "Expectation Failed",
        418 => "I'm a teapot",
        421 => "Misdirected Request",
        422 => "Unprocessable Entity",
        423 => "Locked",
        424 => "Failed Dependency",
        425 => "Too Early",
        426 => "Upgrade Required",
        428 => "Precondition Required",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        451 => "Unavailable For Legal Reasons",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        502 => "Bad Gateway",
        503 => "Service Unavailable",
        504 => "Gateway Timeout",
        505 => "HTTP Version Not Supported",
        506 => "Variant Also Negotiates",
        507 => "Insufficient Storage",
        508 => "Loop Detected",
        510 => "Not Extended",
        511 => "Network Authentication Required",
        _ => "Unknown",
    }
}

fn handle_method_response(
    response: &CompiledMethodResponse,
    req: &Request,
) -> Result<(u16, serde_json::Value), ()> {
    match response {
        CompiledMethodResponse::Response { status, body } => Ok((*status, body.clone())),
        CompiledMethodResponse::Script { script } => {
            match rjscript::evaluator::engine::driver::eval_script(&script, req) {
                Ok((code, val)) => Ok((code, RJSValue::rjs_to_json(&val))),
                Err(err) => {
                    error!("Evaluation error: {}", err);
                    Err(())
                }
            }
        }
    }
}

async fn read_http_request(stream: &mut TcpStream) -> io::Result<Vec<u8>> {
    let mut data = Vec::new();
    let mut buf = [0u8; 1024];

    // Read until we find the header terminator.
    loop {
        let n = stream.read(&mut buf).await?;
        if n == 0 {
            break;
        }
        data.extend_from_slice(&buf[..n]);
        if data.windows(4).any(|window| window == b"\r\n\r\n") {
            break;
        }
    }

    // Determine if there is a Content-Length header to know if the body is complete.
    let request_str = String::from_utf8_lossy(&data).to_string();
    if let Some(header_end) = request_str.find("\r\n\r\n") {
        // Parse the header section to extract Content-Length.
        let headers_section = &request_str[..header_end];
        let mut content_length = 0;
        for line in headers_section.lines().skip(1) {
            // Skip the request line.
            if let Some(idx) = line.find(':') {
                let key = line[..idx].trim();
                let value = line[idx + 1..].trim();
                if key.eq_ignore_ascii_case("Content-Length") {
                    content_length = value.parse::<usize>().unwrap_or(0);
                }
            }
        }

        // Calculate how many body bytes have already been read.
        let body_start = header_end + 4;
        let current_body_len = data.len().saturating_sub(body_start);
        if current_body_len < content_length {
            // Read the remaining body bytes.
            let remaining = content_length - current_body_len;
            let mut body_buf = vec![0; remaining];
            stream.read_exact(&mut body_buf).await?;
            data.extend(body_buf);
        }
    }
    Ok(data)
}

pub async fn handle_client(
    mut stream: TcpStream,
    routes: Option<RoutesData>,
) -> Result<(), Box<dyn std::error::Error>> {
    let data = read_http_request(&mut stream).await?;
    let (method, raw_path, mut req) = parse_http_request(&data);

    // Handle CORS preflight requests with a very permissive policy for easier testing.
    if method.eq_ignore_ascii_case("OPTIONS") {
        let cors_response = "HTTP/1.1 204 No Content\r\n\
Access-Control-Allow-Origin: *\r\n\
Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS\r\n\
Access-Control-Allow-Headers: *\r\n\
Access-Control-Allow-Credentials: true\r\n\
Access-Control-Max-Age: 86400\r\n\r\n";
        stream.write_all(cors_response.as_bytes()).await?;
        return Ok(());
    }

    if routes.is_none() {
        stream.write_all(SERVICE_UNAVAILABLE.as_bytes()).await?;
        return Ok(());
    }
    let routes = routes.unwrap();

    if let Some((response, route_params)) = find_route(
        &routes.static_routes,
        &routes.dynamic_root,
        &raw_path,
        &method,
    ) {
        req.route_params = route_params;

        let result = handle_method_response(&response, &req);

        match result {
            Ok((response_code, response_value)) => {
                let reason = reason_phrase(response_code);
                let mut response_code_string = format!("HTTP/1.1 {} {}\r\n", response_code, reason);

                response_code_string.push_str("Access-Control-Allow-Origin: *\r\n");
                response_code_string.push_str(
                    "Access-Control-Allow-Methods: GET, POST, PUT, PATCH, DELETE, OPTIONS\r\n",
                );
                response_code_string.push_str("Access-Control-Allow-Headers: *\r\n");
                response_code_string.push_str("Access-Control-Allow-Credentials: true\r\n");

                response_code_string.push_str("Content-Type: application/json\r\n\r\n");
                let final_resp = format!("{}{}", response_code_string, response_value.to_string());
                stream.write_all(final_resp.as_bytes()).await?;
            }
            Err(_) => {
                stream.write_all(INTERNAL_SERVER_ERROR.as_bytes()).await?;
            }
        }
    } else {
        stream.write_all(NOT_FOUND.as_bytes()).await?;
    }
    Ok(())
}
