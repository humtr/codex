//! Minimal loopback-only HTTP/1.1 client for `chatgpt-provider`'s local `/control/*` plane.
//!
//! Zero async runtime or TLS dependency: enforces literal loopback addresses (`127.0.0.0/8`,
//! `::1`, or `localhost` without external DNS), reads `<state_dir>/control.token` with
//! `O_NOFOLLOW` + parent symlink/UID check + `0600` mode verification, handles `Content-Length`
//! and incremental `Transfer-Encoding: chunked` strictly per RFC 9112, retries `EINTR`, and
//! caps response bodies at 512 KiB.

use serde_json::{json, Value};
use std::fs::{self, OpenOptions};
use std::io::{self, Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;
use std::time::Duration;

pub const MAX_HTTP_RESPONSE_BYTES: usize = 512 * 1024;
const MAX_HTTP_HEADER_BYTES: usize = 32 * 1024;
const MAX_CONTROL_TOKEN_BYTES: u64 = 4096;
const MIN_CONTROL_TOKEN_LEN: usize = 32;
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
/// Never issues external DNS queries: accepts only literal loopback IPs (`127.0.0.0/8`,
/// `[::1]`) and `localhost` with a non-zero port.
pub fn parse_loopback_base_url(raw: &str) -> Result<LoopbackEndpoint, HttpClientError> {
    let trimmed = raw.trim().trim_end_matches('/');
    let Some(rest) = trimmed.strip_prefix("http://") else {
        return Err(HttpClientError::Config(
            "chatgpt_provider_url_invalid",
            "CHATGPT_PROVIDER_URL must use http:// on a loopback address".to_owned(),
        ));
    };
    if rest.is_empty()
        || rest.contains('/')
        || rest.contains('@')
        || rest.contains('?')
        || rest.contains('#')
        || rest.bytes().any(|b| b <= 0x20 || b == 0x7F)
    {
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
        if host.is_empty() || host.contains(':') {
            return Err(HttpClientError::Config(
                "chatgpt_provider_url_invalid",
                "invalid host in CHATGPT_PROVIDER_URL".to_owned(),
            ));
        }
        if host.eq_ignore_ascii_case("localhost") {
            SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
        } else {
            return Err(HttpClientError::Config(
                "chatgpt_provider_non_loopback_rejected",
                "CHATGPT_PROVIDER_URL must point to a literal loopback address (127.0.0.1, ::1, or localhost)"
                    .to_owned(),
            ));
        }
    } else {
        return Err(HttpClientError::Config(
            "chatgpt_provider_url_invalid",
            "CHATGPT_PROVIDER_URL must include explicit :<port>".to_owned(),
        ));
    };

    if addr.port() == 0 {
        return Err(HttpClientError::Config(
            "chatgpt_provider_url_invalid",
            "CHATGPT_PROVIDER_URL port must be 1..=65535".to_owned(),
        ));
    }

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
    let euid = unsafe { libc::geteuid() };
    if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
        if let Ok(pmeta) = fs::symlink_metadata(parent) {
            if pmeta.file_type().is_symlink() || !pmeta.is_dir() || pmeta.uid() != euid {
                return Err(HttpClientError::Config(
                    "chatgpt_control_token_unsafe",
                    "control token parent directory must be a non-symlink user-owned directory"
                        .to_owned(),
                ));
            }
        }
    }

    let file = OpenOptions::new()
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
        || meta.uid() != euid
        || meta.permissions().mode() & 0o077 != 0
        || meta.len() > MAX_CONTROL_TOKEN_BYTES
    {
        return Err(HttpClientError::Config(
            "chatgpt_control_token_unsafe",
            "control token must be an owner-only 0600 regular file".to_owned(),
        ));
    }
    let mut raw = String::new();
    file.take(MAX_CONTROL_TOKEN_BYTES + 1)
        .read_to_string(&mut raw)
        .map_err(|e| {
            HttpClientError::Unavailable(
                "chatgpt_control_token_unavailable",
                format!("cannot read control token: {e}"),
            )
        })?;
    if raw.len() as u64 > MAX_CONTROL_TOKEN_BYTES {
        return Err(HttpClientError::Config(
            "chatgpt_control_token_unsafe",
            "control token exceeded 4 KiB bound".to_owned(),
        ));
    }
    let token = raw.trim().to_owned();
    if token.len() < MIN_CONTROL_TOKEN_LEN || !token.bytes().all(|b| b.is_ascii_graphic()) {
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
    if method.is_empty() || method.len() > 16 || !method.bytes().all(|b| b.is_ascii_uppercase()) {
        return Err(HttpClientError::Config(
            "chatgpt_request_method_invalid",
            "invalid HTTP method".to_owned(),
        ));
    }
    if !path_and_query.starts_with('/') || path_and_query.bytes().any(|b| b <= 0x20 || b == 0x7F) {
        return Err(HttpClientError::Config(
            "chatgpt_request_path_invalid",
            "invalid HTTP request path".to_owned(),
        ));
    }
    if let Some(tok) = bearer_token {
        if tok.is_empty() || !tok.bytes().all(|b| b.is_ascii_graphic()) {
            return Err(HttpClientError::Config(
                "chatgpt_control_token_invalid",
                "invalid bearer token characters".to_owned(),
            ));
        }
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
#[allow(dead_code)]
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

fn read_chunk_retry<R: Read>(stream: &mut R, buf: &mut [u8]) -> io::Result<usize> {
    loop {
        match stream.read(buf) {
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            other => return other,
        }
    }
}

fn read_http_response<R: Read>(stream: &mut R) -> Result<HttpResponse, HttpClientError> {
    let mut raw = Vec::new();
    let mut buf = [0u8; 8192];

    let (status, content_length, is_chunked, mut rest) = {
        let mut info_skips = 0u8;
        loop {
            let header_end = loop {
                if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                    if pos > MAX_HTTP_HEADER_BYTES {
                        return Err(HttpClientError::TransportLost(
                            "provider HTTP headers exceeded 32 KiB limit".to_owned(),
                        ));
                    }
                    break pos;
                }
                if raw.len() > MAX_HTTP_HEADER_BYTES {
                    return Err(HttpClientError::TransportLost(
                        "provider HTTP headers exceeded 32 KiB limit".to_owned(),
                    ));
                }
                match read_chunk_retry(stream, &mut buf) {
                    Ok(0) => {
                        return Err(HttpClientError::TransportLost(
                            "provider closed connection before HTTP headers".to_owned(),
                        ));
                    }
                    Ok(n) => raw.extend_from_slice(&buf[..n]),
                    Err(e) => {
                        return Err(HttpClientError::TransportLost(format!(
                            "error reading provider HTTP headers: {e}"
                        )));
                    }
                }
            };

            let head = std::str::from_utf8(&raw[..header_end]).map_err(|_| {
                HttpClientError::TransportLost("non-UTF8 HTTP response headers".to_owned())
            })?;
            let mut lines = head.split("\r\n");
            let status_line = lines.next().unwrap_or("");
            let status: u16 = status_line
                .split_whitespace()
                .nth(1)
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| {
                    HttpClientError::TransportLost(format!(
                        "malformed HTTP status line: {status_line}"
                    ))
                })?;

            // Skip `1xx` informational responses (e.g. `100 Continue`) per RFC 9110.
            if (100..200).contains(&status) && status != 101 {
                info_skips += 1;
                if info_skips > 8 {
                    return Err(HttpClientError::TransportLost(
                        "too many 1xx informational HTTP responses".to_owned(),
                    ));
                }
                raw.drain(..header_end + 4);
                continue;
            }

            let mut content_length: Option<usize> = None;
            let mut is_chunked = false;
            for line in lines {
                if let Some((k, v)) = line.split_once(':') {
                    let key = k.trim().to_ascii_lowercase();
                    let val = v.trim();
                    if key == "content-length" {
                        let parsed = val.parse::<usize>().map_err(|_| {
                            HttpClientError::TransportLost(
                                "malformed Content-Length header".to_owned(),
                            )
                        })?;
                        if let Some(prev) = content_length {
                            if prev != parsed {
                                return Err(HttpClientError::TransportLost(
                                    "conflicting duplicate Content-Length headers".to_owned(),
                                ));
                            }
                        }
                        content_length = Some(parsed);
                    } else if key == "transfer-encoding" {
                        let codings: Vec<String> = val
                            .split(',')
                            .map(|v| v.trim().to_ascii_lowercase())
                            .collect();
                        if codings.len() != 1 || codings[0] != "chunked" || is_chunked {
                            return Err(HttpClientError::TransportLost(
                                "unsupported or malformed Transfer-Encoding".to_owned(),
                            ));
                        }
                        is_chunked = true;
                    }
                }
            }

            if is_chunked && content_length.is_some() {
                return Err(HttpClientError::TransportLost(
                    "conflicting Content-Length and Transfer-Encoding headers".to_owned(),
                ));
            }

            let remainder = raw[header_end + 4..].to_vec();
            break (status, content_length, is_chunked, remainder);
        }
    };

    let body_bytes = if is_chunked {
        read_chunked_body(stream, rest)?
    } else if let Some(cl) = content_length {
        if cl > MAX_HTTP_RESPONSE_BYTES {
            return Err(HttpClientError::TransportLost(
                "provider response exceeded 512 KiB bound".to_owned(),
            ));
        }
        while rest.len() < cl {
            match read_chunk_retry(stream, &mut buf) {
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
        rest
    } else {
        loop {
            if rest.len() > MAX_HTTP_RESPONSE_BYTES {
                return Err(HttpClientError::TransportLost(
                    "provider response exceeded 512 KiB bound".to_owned(),
                ));
            }
            match read_chunk_retry(stream, &mut buf) {
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

fn ensure_crlf_line<R: Read>(
    stream: &mut R,
    data: &mut Vec<u8>,
    scratch: &mut [u8],
    limit: usize,
) -> Result<usize, HttpClientError> {
    loop {
        if let Some(pos) = data.windows(2).position(|w| w == b"\r\n") {
            if pos > limit {
                return Err(HttpClientError::TransportLost(
                    "chunked framing line exceeded limit".to_owned(),
                ));
            }
            return Ok(pos);
        }
        if data.len() > limit {
            return Err(HttpClientError::TransportLost(
                "chunked framing line exceeded limit".to_owned(),
            ));
        }
        match read_chunk_retry(stream, scratch) {
            Ok(0) => {
                return Err(HttpClientError::TransportLost(
                    "truncated chunked HTTP response".to_owned(),
                ));
            }
            Ok(n) => data.extend_from_slice(&scratch[..n]),
            Err(e) => {
                return Err(HttpClientError::TransportLost(format!(
                    "error reading chunked HTTP response: {e}"
                )));
            }
        }
    }
}

/// Incrementally read and decode an HTTP/1.1 `Transfer-Encoding: chunked` body without
/// waiting for TCP EOF after the terminating `0\r\n` trailer section and with overflow-safe
/// bounds checks.
fn read_chunked_body<R: Read>(
    stream: &mut R,
    mut buf_in: Vec<u8>,
) -> Result<Vec<u8>, HttpClientError> {
    let mut out = Vec::new();
    let mut scratch = [0u8; 8192];
    const MAX_CHUNK_LINE: usize = 1024;
    const MAX_TRAILER_BYTES: usize = 8192;

    loop {
        let crlf_pos = ensure_crlf_line(stream, &mut buf_in, &mut scratch, MAX_CHUNK_LINE)?;
        let size_line = std::str::from_utf8(&buf_in[..crlf_pos])
            .map_err(|_| HttpClientError::TransportLost("invalid chunk size line".to_owned()))?;
        let hex_part = size_line.split(';').next().unwrap_or("").trim();
        if hex_part.is_empty() || !hex_part.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(HttpClientError::TransportLost(
                "invalid chunk size hex".to_owned(),
            ));
        }
        let size = usize::from_str_radix(hex_part, 16)
            .map_err(|_| HttpClientError::TransportLost("invalid chunk size hex".to_owned()))?;
        buf_in.drain(..crlf_pos + 2);

        if size == 0 {
            // Consume optional trailer headers up to the final empty line (`\r\n`).
            let mut trailer_total = 0usize;
            loop {
                let tpos = ensure_crlf_line(stream, &mut buf_in, &mut scratch, MAX_TRAILER_BYTES)?;
                buf_in.drain(..tpos + 2);
                if tpos == 0 {
                    break;
                }
                trailer_total = trailer_total.saturating_add(tpos + 2);
                if trailer_total > MAX_TRAILER_BYTES {
                    return Err(HttpClientError::TransportLost(
                        "chunked trailer section exceeded limit".to_owned(),
                    ));
                }
            }
            break;
        }

        if size > MAX_HTTP_RESPONSE_BYTES
            || out
                .len()
                .checked_add(size)
                .is_none_or(|total| total > MAX_HTTP_RESPONSE_BYTES)
        {
            return Err(HttpClientError::TransportLost(
                "chunked provider response exceeded 512 KiB bound".to_owned(),
            ));
        }
        let needed = size
            .checked_add(2)
            .ok_or_else(|| HttpClientError::TransportLost("chunked size overflow".to_owned()))?;
        while buf_in.len() < needed {
            match read_chunk_retry(stream, &mut scratch) {
                Ok(0) => {
                    return Err(HttpClientError::TransportLost(
                        "truncated chunk payload".to_owned(),
                    ));
                }
                Ok(n) => buf_in.extend_from_slice(&scratch[..n]),
                Err(e) => {
                    return Err(HttpClientError::TransportLost(format!(
                        "error reading chunk payload: {e}"
                    )));
                }
            }
        }
        if &buf_in[size..size + 2] != b"\r\n" {
            return Err(HttpClientError::TransportLost(
                "missing CRLF after chunk".to_owned(),
            ));
        }
        out.extend_from_slice(&buf_in[..size]);
        buf_in.drain(..size + 2);
    }

    Ok(out)
}

#[cfg(test)]
fn decode_chunked(input: &[u8]) -> Result<Vec<u8>, HttpClientError> {
    let mut cursor = io::Cursor::new(input);
    read_chunked_body(&mut cursor, Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_parser_accepts_loopback_and_rejects_remote_hosts_offline() {
        let ok = parse_loopback_base_url("http://127.0.0.1:4319/").unwrap();
        assert_eq!(ok.canonical_url, "http://127.0.0.1:4319");
        let ok6 = parse_loopback_base_url("http://[::1]:4319").unwrap();
        assert_eq!(ok6.canonical_url, "http://[::1]:4319");
        let ok_lh = parse_loopback_base_url("http://localhost:4319").unwrap();
        assert!(ok_lh.addr.ip().is_loopback());

        assert!(parse_loopback_base_url("https://127.0.0.1:4319").is_err());
        assert!(parse_loopback_base_url("http://0.0.0.0:4319").is_err());
        assert!(parse_loopback_base_url("http://8.8.8.8:4319").is_err());
        assert!(parse_loopback_base_url("http://127.0.0.1:0").is_err());
        assert!(parse_loopback_base_url("http://localhost:0").is_err());
        assert!(parse_loopback_base_url("http://127.0.0.1:4319/extra").is_err());

        // Must fail closed offline without issuing DNS!
        let err = parse_loopback_base_url("http://example.invalid:4319").unwrap_err();
        assert_eq!(err.code(), "chatgpt_provider_non_loopback_rejected");
    }

    #[test]
    fn chunked_decoder_handles_overflow_sign_truncation_and_100_continue() {
        let raw = b"5\r\n{\"ok\"\r\n:6\r\n:true}\r\n0\r\n\r\n";
        assert!(decode_chunked(raw).is_err());
        // Overflow attempt must error cleanly (never panic or wrap!)
        assert!(decode_chunked(b"ffffffffffffffff\r\nabcd\r\n0\r\n\r\n").is_err());
        // Signed hex `+5` must be rejected per RFC 9112
        assert!(decode_chunked(b"+5\r\nhello\r\n0\r\n\r\n").is_err());
        // Missing final CRLF after `0\r\n` must be rejected
        assert!(decode_chunked(b"5\r\nhello\r\n0\r\n").is_err());
        let valid = b"5\r\n{\"ok\"\r\n6\r\n:true}\r\n0\r\n\r\n";
        assert_eq!(decode_chunked(valid).unwrap(), br#"{"ok":true}"#);
        assert_eq!(url_encode("g-p/1?a=b"), "g-p%2F1%3Fa%3Db");

        // 100 Continue followed by 200 OK chunked
        let mut cursor = io::Cursor::new(
            b"HTTP/1.1 100 Continue\r\n\r\nHTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n"
                .as_slice(),
        );
        let resp = read_http_response(&mut cursor).unwrap();
        assert_eq!(resp.status, 200);
        assert_eq!(resp.json, Some(json!({})));

        // Conflicting Content-Length + Transfer-Encoding rejected
        let mut cl_te = io::Cursor::new(
            b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nTransfer-Encoding: chunked\r\n\r\n2\r\n{}\r\n0\r\n\r\n"
                .as_slice(),
        );
        assert!(read_http_response(&mut cl_te).is_err());
    }
}
