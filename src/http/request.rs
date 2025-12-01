use serde_json::Value;
use tracing::debug;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Request {
    pub body: Value,
    pub query_params: HashMap<String, String>,
    pub route_params: HashMap<String, String>,
    pub headers: HashMap<String, String>,
}

impl Request {
    pub fn new(
        body: Value,
        query_params: HashMap<String, String>,
        route_params: HashMap<String, String>,
        headers: HashMap<String, String>,
    ) -> Self {
        Self {
            body,
            query_params,
            route_params,
            headers,
        }
    }
}

pub fn parse_http_request(buffer: &[u8]) -> (String, String, Request) {
    let request_str = String::from_utf8_lossy(buffer).to_string();

    // Find the end of the header section (denoted by \r\n\r\n)
    let header_end = request_str.find("\r\n\r\n").unwrap_or(request_str.len());
    let headers_part = &request_str[..header_end];

    // Split into lines. The first line is the request line.
    let mut lines = headers_part.lines();
    let request_line = lines.next().unwrap_or("");
    
    // Parse request line (e.g., "GET /path?query=val HTTP/1.1")
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let full_path = parts.next().unwrap_or("/").to_string();
    
    // Parse query parameters from the URL path.
    let (raw_path, query_params) = if let Some(idx) = full_path.find('?') {
        let path = full_path[..idx].to_string();
        let query_str = &full_path[idx + 1..];
        let params = query_str
            .split('&')
            .filter_map(|pair| {
                let mut kv = pair.splitn(2, '=');
                let key = kv.next()?.to_string();
                let value = kv.next().unwrap_or("").to_string();
                Some((key, value))
            })
            .collect();
        (path, params)
    } else {
        (full_path, HashMap::new())
    };
    
    // Parse headers.
    let mut headers = HashMap::new();
    for line in lines {
        if let Some(idx) = line.find(':') {
            let key = line[..idx].trim().to_string();
            let value = line[idx + 1..].trim().to_string();
            headers.insert(key, value);
        }
    }
    
    // Parse body.
    let body_str = &request_str[header_end + 4..];
    let body_json = serde_json::from_str(body_str).unwrap_or(serde_json::Value::Null);

    debug!("Method: {}", method);
    debug!("Path: {}", raw_path);
    debug!("Headers: {:?}", headers);
    debug!("Body: {}", body_json);

    let request = Request::new(body_json, query_params, HashMap::new(), headers);
    (method, raw_path, request)
}
