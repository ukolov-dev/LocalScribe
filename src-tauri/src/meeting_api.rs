use crate::{
    errors::{invalid, AppError, AppResult},
    integration,
    types::MeetingSummary,
};
use serde::Deserialize;
use serde_json::json;
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    sync::{mpsc, Arc, Mutex},
    thread,
    time::{Duration, Instant},
};
use tauri::AppHandle;
use uuid::Uuid;

pub const API_HOST: &str = "127.0.0.1";
pub const API_PORT: u16 = 17_865;
const MAX_HEADER_BYTES: usize = 16 * 1024;
const MAX_BODY_BYTES: usize = 1024 * 1024;
const CONNECTION_WORKERS: usize = 4;
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);

#[derive(Debug)]
struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SummaryRequest {
    summary: String,
    model: String,
    generated_at: String,
}

pub fn start(app: &AppHandle) -> AppResult<()> {
    let root = integration::root_dir(app)?;
    let token = load_or_create_token(&root)?;
    let listener = TcpListener::bind((API_HOST, API_PORT)).map_err(|error| {
        AppError::Config(format!(
            "LocalScribe API could not bind to {API_HOST}:{API_PORT}: {error}"
        ))
    })?;
    thread::Builder::new()
        .name("localscribe-meeting-api".to_string())
        .spawn(move || serve(listener, root, token))
        .map_err(|error| AppError::Unexpected(error.to_string()))?;
    Ok(())
}

fn serve(listener: TcpListener, root: PathBuf, token: String) {
    let (sender, receiver) = mpsc::sync_channel::<TcpStream>(CONNECTION_WORKERS);
    let receiver = Arc::new(Mutex::new(receiver));
    for _ in 0..CONNECTION_WORKERS {
        let receiver = Arc::clone(&receiver);
        let root = root.clone();
        let token = token.clone();
        thread::spawn(move || loop {
            let incoming = {
                let Ok(receiver) = receiver.lock() else {
                    return;
                };
                receiver.recv()
            };
            let Ok(mut stream) = incoming else { return };
            if stream.set_write_timeout(Some(REQUEST_TIMEOUT)).is_err() {
                continue;
            }
            if let Err(error) = handle_connection(&mut stream, &root, &token) {
                let _ = write_json(
                    &mut stream,
                    500,
                    &json!({ "error": { "code": error.code(), "message": error.to_string() } }),
                );
            }
        });
    }
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                // A full queue closes the new connection instead of blocking the accept loop.
                let _ = sender.try_send(stream);
            }
            Err(error) => eprintln!("LocalScribe API connection failed: {error}"),
        }
    }
}

fn handle_connection(stream: &mut TcpStream, root: &Path, token: &str) -> AppResult<()> {
    let request = match read_request(stream) {
        Ok(request) => request,
        Err(error) => {
            write_json(
                stream,
                400,
                &json!({ "error": { "code": error.code(), "message": error.to_string() } }),
            )?;
            return Ok(());
        }
    };

    let authorization = request
        .headers
        .get("authorization")
        .and_then(|value| value.strip_prefix("Bearer "));
    if !authorization
        .is_some_and(|candidate| constant_time_eq(candidate.as_bytes(), token.as_bytes()))
    {
        write_json(
            stream,
            401,
            &json!({ "error": { "code": "UNAUTHORIZED", "message": "LocalScribe API token is required." } }),
        )?;
        return Ok(());
    }

    route(stream, root, request)
}

fn route(stream: &mut TcpStream, root: &Path, request: Request) -> AppResult<()> {
    let segments = request
        .path
        .trim_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();

    match (request.method.as_str(), segments.as_slice()) {
        ("GET", ["v1", "health"]) => {
            write_json(stream, 200, &json!({ "status": "ok", "schemaVersion": 1 }))?
        }
        ("GET", ["v1", "meetings"]) => {
            let meetings = integration::list_meetings_in_dir(root)?;
            write_json(stream, 200, &json!({ "meetings": meetings }))?;
        }
        ("GET", ["v1", "meetings", id]) => {
            let detail = match integration::meeting_in_dir(root, id) {
                Ok(detail) => detail,
                Err(AppError::Io(_)) => {
                    write_not_found(stream)?;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            write_json(stream, 200, &detail)?;
        }
        ("PUT", ["v1", "meetings", id, "summary"]) => {
            let payload: SummaryRequest = serde_json::from_slice(&request.body)
                .map_err(|_| invalid("Summary request must be valid JSON."))?;
            let detail = match integration::save_summary_in_dir(
                root,
                id,
                MeetingSummary {
                    text: payload.summary,
                    model: payload.model,
                    generated_at: payload.generated_at,
                },
            ) {
                Ok(detail) => detail,
                Err(AppError::Io(_)) => {
                    write_not_found(stream)?;
                    return Ok(());
                }
                Err(error) => return Err(error),
            };
            write_json(stream, 200, &detail)?;
        }
        _ => write_not_found(stream)?,
    }
    Ok(())
}

fn write_not_found(stream: &mut TcpStream) -> AppResult<()> {
    write_json(
        stream,
        404,
        &json!({ "error": { "code": "NOT_FOUND", "message": "LocalScribe API route was not found." } }),
    )
}

fn read_request(stream: &mut TcpStream) -> AppResult<Request> {
    read_request_until(stream, Instant::now() + REQUEST_TIMEOUT)
}

fn read_request_until(stream: &mut TcpStream, deadline: Instant) -> AppResult<Request> {
    let mut bytes = Vec::new();
    let mut chunk = [0_u8; 4096];
    let header_end = loop {
        let read = read_before_deadline(stream, &mut chunk, deadline)?;
        if read == 0 {
            return Err(invalid("Incomplete HTTP request."));
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() > MAX_HEADER_BYTES + MAX_BODY_BYTES {
            return Err(invalid("HTTP request exceeded the size limit."));
        }
        if let Some(index) = find_bytes(&bytes, b"\r\n\r\n") {
            if index + 4 > MAX_HEADER_BYTES {
                return Err(invalid("HTTP headers exceeded the size limit."));
            }
            break index + 4;
        }
        if bytes.len() > MAX_HEADER_BYTES {
            return Err(invalid("HTTP headers exceeded the size limit."));
        }
    };

    let header = std::str::from_utf8(&bytes[..header_end])
        .map_err(|_| invalid("HTTP headers must be UTF-8."))?;
    let mut lines = header.split("\r\n");
    let request_line = lines
        .next()
        .ok_or_else(|| invalid("Missing HTTP request line."))?;
    let mut request_parts = request_line.split_whitespace();
    let method = request_parts.next().unwrap_or_default();
    let path = request_parts.next().unwrap_or_default();
    let version = request_parts.next().unwrap_or_default();
    if request_parts.next().is_some()
        || !matches!(method, "GET" | "PUT")
        || !path.starts_with('/')
        || path.contains('?')
        || version != "HTTP/1.1"
    {
        return Err(invalid("Unsupported HTTP request line."));
    }
    let method = method.to_string();
    let path = path.to_string();

    let mut headers = HashMap::new();
    for line in lines.filter(|line| !line.is_empty()) {
        let (name, value) = line
            .split_once(':')
            .ok_or_else(|| invalid("Malformed HTTP header."))?;
        let normalized = name.trim().to_ascii_lowercase();
        if normalized.is_empty() || headers.contains_key(&normalized) {
            return Err(invalid("Duplicate or empty HTTP header."));
        }
        headers.insert(normalized, value.trim().to_string());
    }
    if headers.contains_key("transfer-encoding") {
        return Err(invalid("Chunked HTTP requests are not supported."));
    }
    let content_length = headers
        .get("content-length")
        .map(|value| value.parse::<usize>())
        .transpose()
        .map_err(|_| invalid("Invalid Content-Length header."))?
        .unwrap_or(0);
    if content_length > MAX_BODY_BYTES {
        return Err(invalid("HTTP body exceeded the size limit."));
    }

    while bytes.len() - header_end < content_length {
        let read = read_before_deadline(stream, &mut chunk, deadline)?;
        if read == 0 {
            return Err(invalid("Incomplete HTTP request body."));
        }
        bytes.extend_from_slice(&chunk[..read]);
        if bytes.len() - header_end > MAX_BODY_BYTES {
            return Err(invalid("HTTP body exceeded the size limit."));
        }
    }
    if bytes.len() - header_end != content_length {
        return Err(invalid("HTTP request contains unexpected trailing bytes."));
    }

    Ok(Request {
        method,
        path,
        headers,
        body: bytes[header_end..].to_vec(),
    })
}

fn read_before_deadline(
    stream: &mut TcpStream,
    buffer: &mut [u8],
    deadline: Instant,
) -> AppResult<usize> {
    let remaining = deadline
        .checked_duration_since(Instant::now())
        .filter(|remaining| !remaining.is_zero())
        .ok_or_else(|| invalid("HTTP request timed out."))?;
    stream.set_read_timeout(Some(remaining))?;
    Ok(stream.read(buffer)?)
}

fn write_json(
    stream: &mut TcpStream,
    status: u16,
    payload: &impl serde::Serialize,
) -> AppResult<()> {
    let body = serde_json::to_vec(payload)?;
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        _ => "Internal Server Error",
    };
    write!(
        stream,
        "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nConnection: close\r\n\r\n",
        body.len()
    )?;
    stream.write_all(&body)?;
    stream.flush()?;
    Ok(())
}

fn load_or_create_token(root: &Path) -> AppResult<String> {
    let path = root.join("api-token");
    match fs::read_to_string(&path) {
        Ok(value) => validate_token(value.trim()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            let value = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
            create_token_file(&path, &value)?;
            Ok(value)
        }
        Err(error) => Err(error.into()),
    }
}

fn validate_token(value: &str) -> AppResult<String> {
    if value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        Ok(value.to_string())
    } else {
        Err(AppError::Config(
            "LocalScribe API token is invalid; remove integration/api-token to regenerate it."
                .to_string(),
        ))
    }
}

#[cfg(unix)]
fn create_token_file(path: &Path, value: &str) -> AppResult<()> {
    use std::os::unix::fs::OpenOptionsExt;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(value.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(not(unix))]
fn create_token_file(path: &Path, value: &str) -> AppResult<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    file.write_all(value.as_bytes())?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    if left.len() != right.len() {
        return false;
    }
    left.iter()
        .zip(right)
        .fold(0_u8, |difference, (left, right)| {
            difference | (left ^ right)
        })
        == 0
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_tokens_without_short_circuiting_contents() {
        assert!(constant_time_eq(b"abcdef", b"abcdef"));
        assert!(!constant_time_eq(b"abcdef", b"abcdeg"));
        assert!(!constant_time_eq(b"abcdef", b"short"));
    }

    #[test]
    fn finds_http_header_boundary() {
        assert_eq!(find_bytes(b"GET / HTTP/1.1\r\n\r\n", b"\r\n\r\n"), Some(14));
    }

    #[test]
    fn validates_generated_token_shape() {
        let value = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
        assert_eq!(validate_token(&value).unwrap(), value);
        assert!(validate_token("too-short").is_err());
    }

    #[test]
    fn serialized_api_error_does_not_include_token() {
        let payload = serde_json::to_string(&json!({
            "error": { "code": "UNAUTHORIZED", "message": "token required" }
        }))
        .unwrap();
        assert!(!payload.contains("secret"));
    }
    #[test]
    fn idle_client_does_not_block_an_authorized_health_request() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        thread::spawn(move || serve(listener, std::env::temp_dir(), "test-token".to_string()));
        let idle = TcpStream::connect(address).unwrap();
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        client
            .write_all(b"GET /v1/health HTTP/1.1\r\nAuthorization: Bearer test-token\r\n\r\n")
            .unwrap();
        let mut response = Vec::new();
        let result = client.read_to_end(&mut response);
        drop(idle);
        result.unwrap();
        assert!(std::str::from_utf8(&response)
            .unwrap()
            .starts_with("HTTP/1.1 200"));
    }

    #[test]
    fn request_deadline_is_shared_by_headers_and_body() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let mut client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        client
            .write_all(b"PUT /v1/meetings/test/summary HTTP/1.1\r\nContent-Length: 2\r\n\r\n")
            .unwrap();
        let sender = thread::spawn(move || {
            thread::sleep(Duration::from_millis(60));
            let _ = client.write_all(b"{");
            thread::sleep(Duration::from_millis(100));
            let _ = client.write_all(b"}");
        });
        let start = Instant::now();
        assert!(read_request_until(&mut server, start + Duration::from_millis(100)).is_err());
        drop(server);
        sender.join().unwrap();
    }

    #[test]
    fn rejects_requests_before_reading_with_an_expired_deadline() {
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let _client = TcpStream::connect(listener.local_addr().unwrap()).unwrap();
        let (mut server, _) = listener.accept().unwrap();
        let error = read_request_until(&mut server, Instant::now()).unwrap_err();
        assert!(error.to_string().contains("timed out"));
    }
}
