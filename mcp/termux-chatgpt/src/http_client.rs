//! Minimal loopback-only HTTP/1.1 client for `chatgpt-provider`'s local `/control/*` plane.
//!
//! Zero async runtime or TLS dependency: enforces loopback-only addresses (`127.0.0.1` / `::1`),
//! reads `<state_dir>/control.token` with `O_NOFOLLOW` + owner UID + `0600` mode verification,
//! handles `Content-Length` and `Transfer-Encoding: chunked`, and caps response bodies at 512 KiB.

use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::net::{IpAddr, SocketAddr, TcpStream, ToSocketAddrs};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::Duration;

pub const MAX_HTTP_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 32 * 1024;
const CONNECT_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone)]
pub struct LoopbackEndpoint {
    pub host_header: String,
    pub addr: SocketAddr,
    pub canonical_url: String,
}

#[derive(Debug)]
pub enum HttpClientError {
    /// Failed before sending an HTTP request (safe to retry without poisoning `call_id`).
    Unavailable(&'static str, String),
    /// Connected and dispatched bytes, then failed/timed out reading response (delivery unknown).
    TransportLost(String),
    /// Invalid local configuration or unsafe token file.
    Config(&'static str, String),
}

impl HttpClientError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Unavailable(c, _) | Self::Config(c, _) => c,
            Self::TransportLost(_) => "chatgpt_provider_transport_lost",
        }
    }

    pub fn detail(&self) -> &str {
        match self {
            Self::Unavailable(_, d) | Self::Config(_, d) | Self::TransportLost(d) => d,
        }
    }
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub status: u16,
    pub json: Option<Value>,
    pub raw_text: String,
}

/// Parse and strictly validate a loopback-only `http://<loopback>:<port>` URL.
pub fn parse_loopback_base_url(raw: &str) -> Result<LoopbackEndpoint, HttpClientError> {
    let trimmed = raw.trim().trim_end_matches('/');
    let Some(rest) = trimmed.strip_prefix("http://") else {
        return Err(HttpClientError::Config(
            "chatgpt_provider_url_invalid",
            "CHATGPT_PROVIDER_URL must use http:// on a loopback address".to_owned(),
        ));
    };
    if rest.is_empty() || rest.contains('/') || rest.contains('@') || rest.contains('?') {
        return Err(HttpClientError::Config(
            "chatgpt_provider_url_invalid",
            "CHATGPT_PROVIDER_URL must contain only http://<host>:<port>".to_owned(),
        ));
    }

    let addr: SocketAddr = if let Ok(direct) = rest.parse::<SocketAddr>() {
        direct
    } else if let Some((host, port_str)) = rest.rsplit_once(':') {
        let port: u16 = port_str.parse().map_err(|_| {
            HttpClientError::Config(
                "chatgpt_provider_url_invalid",
                "invalid port in CHATGPT_PROVIDER_URL".to_owned(),
            )
        })?;
        if host.eq_ignore_ascii_case("localhost") {
            SocketAddr::new(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST), port)
        } else {
            let mut addrs = (host, port).to_socket_addrs().map_err(|e| {
                HttpClientError::Config(
                    "chatgpt_provider_url_invalid",
                    format!("cannot resolve provider host: {e}"),
                )
            })?;
            addrs.next().ok_or_else(|| {
                HttpClientError::Config(
                    "chatgpt_provider_url_invalid",
                    "provider host resolved to no addresses".to_owned(),
                )
            })?
        }
    } else {
        return Err(HttpClientError::Config(
            "chatgpt_provider_url_invalid",
            "CHATGPT_PROVIDER_URL must include explicit :<port>".to_owned(),
        ));
    };

    if !addr.ip().is_loopback() {
        return Err(HttpClientError::Config(
            "chatgpt_provider_non_loopback_rejected",
            "CHATGPT_PROVIDER_URL must point to a loopback address (127.0.0.1 or ::1)".to_owned(),
        ));
    }

    let host_header = match addr.ip() {
        IpAddr::V4(ip) => format!("{ip}:{}", addr.port()),
        IpAddr::V6(ip) => format!("[{ip}]:{}", addr.port()),
    };
    let canonical_url = format!("http://{host_header}");
    Ok(LoopbackEndpoint {
        host_header,
        addr,
        canonical_url,
    })
}

/// Read the private `0600` bearer token from `<state_dir>/control.token` with strict ownership checks.
/// Never includes the token value in any error message.
pub fn read_control_token(path: &Path) -> Result<String, HttpClientError> {
    let mut file = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|e| {
            HttpClientError::Unavailable(
                "chatgpt_control_token_unavailable",
                format!("cannot open control token {}: {e}", path.display()),
            )
        })?;
    let meta = file.metadata().map_err(|e| {
        HttpClientError::Unavailable(
            "chatgpt_control_token_unavailable",
            format!("cannot stat control token: {e}"),
        )
    })?;
    if !meta.is_file()
        || meta.uid() != unsafe { libc::geteuid() }
        || meta.permissions().mode() & 0o077 != 0
        || meta.len() > 4096
    {
        return Err(HttpClientError::Config(
            "chatgpt_control_token_unsafe",
            "control token must be an owner-only 0600 regular file".to_owned(),
        ));
    }
    let mut raw = String::new();
    file.read_to_string(&mut raw).map_err(|e| {
        HttpClientError::Unavailable(
            "chatgpt_control_token_unavailable",
            format!("cannot read control token: {e}"),
        )
    })?;
    let token = raw.trim().to_owned();
    if token.len() < 16 || token.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return Err(HttpClientError::Config(
            "chatgpt_control_token_invalid",
            "control token format invalid".to_owned(),
        ));
    }
    Ok(token)
}

/// Percent-encode a URL path segment or query parameter value (RFC 3986 unreserved set).
pub fn url_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "%{b:02X}");
        }
    }
    out
}

/// Perform a single bounded HTTP/1.1 request to the local `chatgpt-provider`.
pub fn request_json(
    endpoint: &LoopbackEndpoint,
    method: &str,
    path_and_query: &str,
    bearer_token: Option<&str>,
    body: Option<&Value>,
    read_timeout: Duration,
) -> Result<HttpResponse, HttpClientError> {
    if !path_and_query.starts_with('/')
        || path_and_query.contains('\r')
        || path_and_query.contains('\n')
        || path_and_query.contains('\0')
    {
        return Err(HttpClientError::Config(
            "chatgpt_request_path_invalid",
            "invalid HTTP request path".to_owned(),
        ));
    }

    let mut stream = TcpStream::connect_timeout(&endpoint.addr, CONNECT_TIMEOUT).map_err(|e| {
        HttpClientError::Unavailable(
            "chatgpt_provider_unavailable",
            format!(
                "cannot connect to chatgpt-provider at {}: {e}",
                endpoint.canonical_url
            ),
        )
    })?;
    let _ = stream.set_write_timeout(Some(Duration::from_secs(10)));
    let _ = stream.set_read_timeout(Some(read_timeout));

    let body_bytes = match body {
        Some(v) => v.to_string().into_bytes(),
        None => Vec::new(),
    };
    let mut req = format!(
        "{method} {path_and_query} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nAccept: application/json\r\n",
        endpoint.host_header
    );
    if let Some(tok) = bearer_token {
        req.push_str("Authorization: Bearer ");
        req.push_str(tok);
        req.push_str("\r\n");
    }
    if body.is_some() {
        req.push_str("Content-Type: application/json\r\n");
        req.push_str(&format!("Content-Length: {}\r\n", body_bytes.len()));
    }
    req.push_str("\r\n");

    stream
        .write_all(req.as_bytes())
        .and_then(|_| {
            if !body_bytes.is_empty() {
                stream.write_all(&body_bytes)
            } else {
                Ok(())
            }
        })
        .and_then(|_| stream.flush())
        .map_err(|e| {
            HttpClientError::TransportLost(format!("failed sending request to provider: {e}"))
        })?;

    read_http_response(&mut stream)
}

/// Lightweight reachability check (`GET /healthz`) safe to run before claiming a new `call_id`.
pub fn check_healthz(endpoint: &LoopbackEndpoint) -> Result<Value, HttpClientError> {
    let resp = request_json(
        endpoint,
        "GET",
        "/healthz",
        None,
        None,
        Duration::from_secs(5),
    )?;
    if resp.status == 200 {
        Ok(resp.json.unwrap_or_else(|| json!({"ok": true})))
    } else {
        Err(HttpClientError::Unavailable(
            "chatgpt_provider_unhealthy",
            format!("provider /healthz returned HTTP {}", resp.status),
        ))
    }
}

fn read_http_response<R: Read>(stream: &mut R) -> Result<HttpResponse, HttpClientError> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];
    let header_end = loop {
        match stream.read(&mut buf) {
            Ok(0) => {
                return Err(HttpClientError::TransportLost(
                    "provider closed connection before HTTP headers".to_owned(),
                ));
            }
            Ok(n) => {
                raw.extend_from_slice(&buf[..n]);
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    break pos;
                }
                if raw.len() > MAX_HTTP_HEADER_BYTES {
                    return Err(HttpClientError::TransportLost(
                        "provider HTTP headers exceeded 32 KiB limit".to_owned(),
                    ));
                }
            }
            Err(e) => {
                return Err(HttpClientError::TransportLost(format!(
                    "error reading provider HTTP headers: {e}"
                )));
            }
        }
    };

    let head = std::str::from_utf8(&raw[..header_end])
        .map_err(|_| HttpClientError::TransportLost("non-UTF8 HTTP response headers".to_owned()))?;
    let mut lines = head.split("\r\n");
    let status_line = lines.next().unwrap_or("");
    let status: u16 = status_line
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| {
            HttpClientError::TransportLost(format!("malformed HTTP status line: {status_line}"))
        })?;

    let mut content_length: Option<usize> = None;
    let mut is_chunked = false;
    for line in lines {
        if let Some((k, v)) = line.split_once(':') {
            let key = k.trim().to_ascii_lowercase();
            let val = v.trim();
            if key == "content-length" {
                content_length = val.parse::<usize>().ok();
            } else if key == "transfer-encoding" && val.to_ascii_lowercase().contains("chunked") {
                is_chunked = true;
            }
        }
    }

    let mut rest = raw[header_end + 4..].to_vec();
    if let Some(cl) = content_length {
        if cl > MAX_HTTP_RESPONSE_BYTES {
            return Err(HttpClientError::TransportLost(
                "provider response exceeded 512 KiB bound".to_owned(),
            ));
        }
        while rest.len() < cl {
            match stream.read(&mut buf) {
                Ok(0) => {
                    return Err(HttpClientError::TransportLost(
                        "provider closed connection before full Content-Length body".to_owned(),
                    ));
                }
                Ok(n) => rest.extend_from_slice(&buf[..n]),
                Err(e) => {
                    return Err(HttpClientError::TransportLost(format!(
                        "error reading provider response body: {e}"
                    )));
                }
            }
        }
        rest.truncate(cl);
    } else {
        loop {
            if rest.len() > MAX_HTTP_RESPONSE_BYTES + 8192 {
                return Err(HttpClientError::TransportLost(
                    "provider response exceeded 512 KiB bound".to_owned(),
                ));
            }
            match stream.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => rest.extend_from_slice(&buf[..n]),
                Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
                Err(e) => {
                    return Err(HttpClientError::TransportLost(format!(
                        "error reading provider response body: {e}"
                    )));
                }
            }
        }
    }

    let body_bytes = if is_chunked {
        decode_chunked(&rest)?
    } else {
        if rest.len() > MAX_HTTP_RESPONSE_BYTES {
            return Err(HttpClientError::TransportLost(
                "provider response exceeded 512 KiB bound".to_owned(),
            ));
        }
        rest
    };

    let raw_text = String::from_utf8_lossy(&body_bytes).into_owned();
    let json = serde_json::from_slice::<Value>(&body_bytes).ok();
    Ok(HttpResponse {
        status,
        json,
        raw_text,
    })
}

fn decode_chunked(input: &[u8]) -> Result<Vec<u8>, HttpClientError> {
    let mut out = Vec::new();
    let mut pos = 0usize;
    loop {
        let Some(rel) = input[pos..].windows(2).position(|w| w == b"\r\n") else {
            return Err(HttpClientError::TransportLost(
                "truncated chunked HTTP response".to_owned(),
            ));
        };
        let size_line = std::str::from_utf8(&input[pos..pos + rel])
            .map_err(|_| HttpClientError::TransportLost("invalid chunk size line".to_owned()))?;
        let hex_part = size_line.split(';').next().unwrap_or("").trim();
        let size = usize::from_str_radix(hex_part, 16)
            .map_err(|_| HttpClientError::TransportLost("invalid chunk size hex".to_owned()))?;
        pos += rel + 2;
        if size == 0 {
            break;
        }
        if pos + size + 2 > input.len() {
            return Err(HttpClientError::TransportLost(
                "truncated chunk payload".to_owned(),
            ));
        }
        if out.len() + size > MAX_HTTP_RESPONSE_BYTES {
            return Err(HttpClientError::TransportLost(
                "chunked provider response exceeded 512 KiB bound".to_owned(),
            ));
        }
        out.extend_from_slice(&input[pos..pos + size]);
        pos += size;
        if &input[pos..pos + 2] != b"\r\n" {
            return Err(HttpClientError::TransportLost(
                "missing CRLF after chunk".to_owned(),
            ));
        }
        pos += 2;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_parser_accepts_loopback_and_rejects_remote_hosts() {
        let ok = parse_loopback_base_url("http://127.0.0.1:4319/").unwrap();
        assert_eq!(ok.canonical_url, "http://127.0.0.1:4319");
        let ok6 = parse_loopback_base_url("http://[::1]:4319").unwrap();
        assert_eq!(ok6.canonical_url, "http://[::1]:4319");
        let ok_lh = parse_loopback_base_url("http://localhost:4319").unwrap();
        assert!(ok_lh.addr.ip().is_loopback());

        assert!(parse_loopback_base_url("https://127.0.0.1:4319").is_err());
        assert!(parse_loopback_base_url("http://0.0.0.0:4319").is_err());
        assert!(parse_loopback_base_url("http://8.8.8.8:4319").is_err());
        assert!(parse_loopback_base_url("http://127.0.0.1:4319/extra").is_err());
    }

    #[test]
    fn chunked_decoder_and_url_encode_work() {
        let raw = b"5\r\n{\"ok\"\r\n:6\r\n:true}\r\n0\r\n\r\n";
        assert!(decode_chunked(raw).is_err());
        let valid = b"5\r\n{\"ok\"\r\n6\r\n:true}\r\n0\r\n\r\n";
        assert_eq!(decode_chunked(valid).unwrap(), br#"{"ok":true}"#);
        assert_eq!(url_encode("g-p/1?a=b"), "g-p%2F1%3Fa%3Db");
    }
}
