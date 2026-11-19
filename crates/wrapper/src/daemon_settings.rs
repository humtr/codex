use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

const MAX_SETTINGS_BYTES: usize = 1024 * 1024;
const MAX_JSON_DEPTH: usize = 32;
const DEFAULT_SETTINGS_BYTES: &[u8] = b"{\"updater\":{\"autoUpdateEnabled\":false}}\n";

static SETTINGS_TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn inspect_read_only(codex_home: &Path) -> io::Result<()> {
    if codex_home.as_os_str().is_empty() {
        return Ok(());
    }
    match fs::symlink_metadata(codex_home) {
        Ok(meta) if meta.file_type().is_symlink() => {
            if !fs::metadata(codex_home)?.is_dir() {
                return Err(io::Error::other("CODEX_HOME must be a directory"));
            }
        }
        Ok(meta) if meta.is_dir() => {}
        Ok(_) => return Err(io::Error::other("CODEX_HOME must be a directory")),
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    }
    let physical_home = fs::canonicalize(codex_home)?;
    let daemon_dir = physical_home.join("app-server-daemon");
    match fs::symlink_metadata(&daemon_dir) {
        Ok(meta) if meta.file_type().is_dir() => {}
        Ok(_) => {
            return Err(io::Error::other(
                "app-server-daemon path must be a real directory",
            ))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    }
    let settings_path = daemon_dir.join("settings.json");
    match fs::symlink_metadata(&settings_path) {
        Ok(meta) if meta.file_type().is_file() => {
            if metadata_exceeds_limit(&meta) {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            let file = File::open(&settings_path)?;
            let mut bytes = Vec::with_capacity(meta.len() as usize);
            file.take(MAX_SETTINGS_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_SETTINGS_BYTES {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            let _ = update_settings_bytes(Some(&bytes))?;
            Ok(())
        }
        Ok(_) => Err(io::Error::other(
            "app-server-daemon settings.json must be a regular file",
        )),
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

fn metadata_exceeds_limit(meta: &fs::Metadata) -> bool {
    meta.len() > MAX_SETTINGS_BYTES as u64
}

pub(super) fn ensure(codex_home: &Path) -> io::Result<()> {
    if codex_home.as_os_str().is_empty() {
        return Ok(());
    }
    match fs::symlink_metadata(codex_home) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            if !fs::metadata(codex_home)?.is_dir() {
                return Err(io::Error::other("CODEX_HOME must be a directory"));
            }
        }
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => return Err(io::Error::other("CODEX_HOME must be a directory")),
        Err(err) if err.kind() == ErrorKind::NotFound => fs::create_dir_all(codex_home)?,
        Err(err) => return Err(err),
    }

    let physical_home = fs::canonicalize(codex_home)?;
    let daemon_dir = physical_home.join("app-server-daemon");
    match fs::symlink_metadata(&daemon_dir) {
        Ok(metadata) if metadata.file_type().is_dir() => {}
        Ok(_) => {
            return Err(io::Error::other(
                "app-server-daemon path must be a real directory",
            ))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let mut builder = DirBuilder::new();
            builder.mode(0o700);
            match builder.create(&daemon_dir) {
                Ok(()) => {}
                Err(create_err) if create_err.kind() == ErrorKind::AlreadyExists => {
                    let meta = fs::symlink_metadata(&daemon_dir)?;
                    if !meta.file_type().is_dir() {
                        return Err(io::Error::other(
                            "app-server-daemon path must be a real directory",
                        ));
                    }
                }
                Err(create_err) => return Err(create_err),
            }
        }
        Err(err) => return Err(err),
    }

    let settings_path = daemon_dir.join("settings.json");
    let existing = match fs::symlink_metadata(&settings_path) {
        Ok(metadata) if metadata.file_type().is_file() => {
            if metadata.len() > MAX_SETTINGS_BYTES as u64 {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            let file = File::open(&settings_path)?;
            let mut bytes = Vec::with_capacity(metadata.len() as usize);
            file.take(MAX_SETTINGS_BYTES as u64 + 1)
                .read_to_end(&mut bytes)?;
            if bytes.len() > MAX_SETTINGS_BYTES {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json exceeds size limit",
                ));
            }
            Some(bytes)
        }
        Ok(_) => {
            return Err(io::Error::other(
                "app-server-daemon settings.json must be a regular file",
            ))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => None,
        Err(err) => return Err(err),
    };

    let Some(updated) = update_settings_bytes(existing.as_deref())? else {
        return Ok(());
    };

    let sequence = SETTINGS_TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let temp_path = daemon_dir.join(format!(
        ".settings.json.{}.{sequence}.tmp",
        std::process::id()
    ));
    let write_result = (|| -> io::Result<()> {
        let mut temp = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp_path)?;
        temp.write_all(&updated)?;
        temp.sync_all()?;
        drop(temp);
        fs::rename(&temp_path, &settings_path)?;
        File::open(&daemon_dir)?.sync_all()?;
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path);
    }
    write_result
}

const EXISTING_PACKAGE_INDICATORS: &[&str] = &[
    "current",
    "releases",
    "selection-history",
    "auto-update-version",
    "install.lock",
];

const PRIOR_DAEMON_TRACE_FILES: &[&str] = &[
    "daemon.pid",
    "daemon.stderr.log",
    "daemon-updater.pid",
    "daemon-updater.stderr.log",
    "app-server.pid",
    "app-server.stderr.log",
    "app-server-updater.pid",
    "app-server-updater.stderr.log",
];

fn current_uid() -> io::Result<u32> {
    fs::metadata("/proc/self").map(|m| m.uid())
}

fn inspect_owned_real_dir(path: &Path, expected_uid: u32) -> io::Result<Option<bool>> {
    match fs::symlink_metadata(path) {
        Ok(meta) => {
            if meta.file_type().is_symlink() || !meta.is_dir() || meta.uid() != expected_uid {
                return Ok(Some(false));
            }
            Ok(Some(true))
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn ensure_private_real_dir_no_chmod(path: &Path, expected_uid: u32) -> io::Result<bool> {
    if let Some(valid) = inspect_owned_real_dir(path, expected_uid)? {
        return Ok(valid);
    }
    let mut builder = DirBuilder::new();
    builder.mode(0o700);
    match builder.create(path) {
        Ok(()) => Ok(true),
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            Ok(inspect_owned_real_dir(path, expected_uid)? == Some(true))
        }
        Err(err) => Err(err),
    }
}

fn may_exist(path: &Path) -> bool {
    !matches!(fs::symlink_metadata(path), Err(err) if err.kind() == ErrorKind::NotFound)
}

/// Atomically create `$CODEX_HOME/packages/app-server-daemon/current -> standalone_current`
/// on first managed-daemon initialization without overwriting any existing or partial state.
pub(super) fn ensure_default_package_link(
    codex_home: &Path,
    standalone_current: &Path,
    standalone_root: &Path,
) -> io::Result<()> {
    if codex_home.as_os_str().is_empty() {
        return Ok(());
    }
    // Zero-write, zero-fsync fast path when `current` (or broken link) is already present.
    if may_exist(&codex_home.join("packages/app-server-daemon/current")) {
        return Ok(());
    }
    if fs::symlink_metadata(standalone_current).is_err()
        || fs::metadata(standalone_current).is_err()
    {
        return Ok(());
    }

    let physical_home = match fs::canonicalize(codex_home) {
        Ok(path) => path,
        Err(err) if err.kind() == ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if let Ok(canonical_standalone) = fs::canonicalize(standalone_root) {
        if physical_home.starts_with(&canonical_standalone) {
            return Ok(());
        }
    }

    let uid = current_uid()?;
    let packages_dir = physical_home.join("packages");
    let packages_state = inspect_owned_real_dir(&packages_dir, uid)?;
    if packages_state == Some(false) {
        return Ok(());
    }
    let pkg_daemon_dir = packages_dir.join("app-server-daemon");
    if packages_state == Some(true) {
        let daemon_pkg_state = inspect_owned_real_dir(&pkg_daemon_dir, uid)?;
        if daemon_pkg_state == Some(false) {
            return Ok(());
        }
        if daemon_pkg_state == Some(true) {
            for indicator in EXISTING_PACKAGE_INDICATORS {
                if may_exist(&pkg_daemon_dir.join(indicator)) {
                    return Ok(());
                }
            }
        }
    }

    // Upstream 0.170.0 chooses an existing dedicated current first, then modern
    // traces, then legacy traces. Leave the choice to upstream for all of them.
    let state_dir = physical_home.join("app-server-daemon");
    for trace in PRIOR_DAEMON_TRACE_FILES {
        if may_exist(&state_dir.join(trace)) {
            return Ok(());
        }
    }
    if may_exist(&physical_home.join("app-server-control/app-server-control.sock")) {
        return Ok(());
    }

    if !ensure_private_real_dir_no_chmod(&packages_dir, uid)? {
        return Ok(());
    }
    if !ensure_private_real_dir_no_chmod(&pkg_daemon_dir, uid)? {
        return Ok(());
    }

    let current_link = pkg_daemon_dir.join("current");
    match std::os::unix::fs::symlink(standalone_current, &current_link) {
        Ok(()) => {
            File::open(&pkg_daemon_dir)?.sync_all()?;
            Ok(())
        }
        // Another launcher or upstream installer made the selection.
        Err(err) if err.kind() == ErrorKind::AlreadyExists => Ok(()),
        Err(err) => Err(err),
    }
}

fn update_settings_bytes(existing: Option<&[u8]>) -> io::Result<Option<Vec<u8>>> {
    let Some(raw) = existing else {
        return Ok(Some(DEFAULT_SETTINGS_BYTES.to_vec()));
    };
    let mut parser = JsonParser::new(raw);
    let mut root = parser.parse_document()?;
    let JsonValue::Object(ref mut entries) = root else {
        return Err(io::Error::new(
            ErrorKind::InvalidData,
            "app-server-daemon settings.json must be a JSON object",
        ));
    };

    let mut changed = false;
    let mut found_updater = false;
    for (_, updater) in entries.iter_mut().filter(|(key, _)| key == "updater") {
        found_updater = true;
        if matches!(updater, JsonValue::Null) {
            *updater = JsonValue::Object(Vec::new());
            changed = true;
        }
        let JsonValue::Object(fields) = updater else {
            return Err(io::Error::new(
                ErrorKind::InvalidData,
                "app-server-daemon settings.json updater field is not an object",
            ));
        };
        let mut found_flag = false;
        for (_, flag) in fields
            .iter_mut()
            .filter(|(key, _)| key == "autoUpdateEnabled")
        {
            found_flag = true;
            if !matches!(flag, JsonValue::Bool(false)) {
                *flag = JsonValue::Bool(false);
                changed = true;
            }
        }
        if !found_flag {
            fields.push(("autoUpdateEnabled".to_owned(), JsonValue::Bool(false)));
            changed = true;
        }
    }
    if !found_updater {
        entries.push((
            "updater".to_owned(),
            JsonValue::Object(vec![(
                "autoUpdateEnabled".to_owned(),
                JsonValue::Bool(false),
            )]),
        ));
        changed = true;
    }

    if !changed {
        return Ok(None);
    }

    let mut out = String::new();
    root.write_json(&mut out);
    out.push('\n');
    Ok(Some(out.into_bytes()))
}

#[derive(Debug, Clone, PartialEq)]
enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    fn write_json(&self, out: &mut String) {
        match self {
            JsonValue::Null => out.push_str("null"),
            JsonValue::Bool(true) => out.push_str("true"),
            JsonValue::Bool(false) => out.push_str("false"),
            JsonValue::Number(raw) => out.push_str(raw),
            JsonValue::String(text) => write_json_string(text, out),
            JsonValue::Array(items) => {
                out.push('[');
                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    item.write_json(out);
                }
                out.push(']');
            }
            JsonValue::Object(entries) => {
                out.push('{');
                for (index, (key, value)) in entries.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }
                    write_json_string(key, out);
                    out.push(':');
                    value.write_json(out);
                }
                out.push('}');
            }
        }
    }
}

fn write_json_string(text: &str, out: &mut String) {
    use std::fmt::Write as _;

    out.push('"');
    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if (ch as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
    out.push('"');
}

struct JsonParser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> JsonParser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn invalid(msg: &'static str) -> io::Error {
        io::Error::new(ErrorKind::InvalidData, msg)
    }

    fn skip_ws(&mut self) {
        while matches!(
            self.bytes.get(self.offset),
            Some(b' ' | b'\n' | b'\r' | b'\t')
        ) {
            self.offset += 1;
        }
    }

    fn parse_document(&mut self) -> io::Result<JsonValue> {
        let value = self.parse_value(0)?;
        self.skip_ws();
        if self.offset != self.bytes.len() {
            return Err(Self::invalid("trailing data after JSON document"));
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth: usize) -> io::Result<JsonValue> {
        if depth >= MAX_JSON_DEPTH {
            return Err(Self::invalid("JSON nesting exceeds maximum depth"));
        }
        self.skip_ws();
        let Some(&byte) = self.bytes.get(self.offset) else {
            return Err(Self::invalid("unexpected end of JSON"));
        };
        match byte {
            b'{' => self.parse_object(depth + 1),
            b'[' => self.parse_array(depth + 1),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' => self.expect_literal(b"true", JsonValue::Bool(true)),
            b'f' => self.expect_literal(b"false", JsonValue::Bool(false)),
            b'n' => self.expect_literal(b"null", JsonValue::Null),
            b'-' | b'0'..=b'9' => self.parse_number().map(JsonValue::Number),
            _ => Err(Self::invalid("unexpected token in JSON")),
        }
    }

    fn expect_literal(&mut self, literal: &[u8], value: JsonValue) -> io::Result<JsonValue> {
        if self.bytes[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(value)
        } else {
            Err(Self::invalid("invalid JSON literal"))
        }
    }

    fn parse_object(&mut self, depth: usize) -> io::Result<JsonValue> {
        self.offset += 1;
        self.skip_ws();
        let mut entries = Vec::new();
        if self.bytes.get(self.offset) == Some(&b'}') {
            self.offset += 1;
            return Ok(JsonValue::Object(entries));
        }
        loop {
            self.skip_ws();
            if self.bytes.get(self.offset) != Some(&b'"') {
                return Err(Self::invalid("expected JSON object key"));
            }
            let key = self.parse_string()?;
            self.skip_ws();
            if self.bytes.get(self.offset) != Some(&b':') {
                return Err(Self::invalid("expected ':' in JSON object"));
            }
            self.offset += 1;
            let value = self.parse_value(depth)?;
            entries.push((key, value));
            self.skip_ws();
            match self.bytes.get(self.offset) {
                Some(b',') => self.offset += 1,
                Some(b'}') => {
                    self.offset += 1;
                    return Ok(JsonValue::Object(entries));
                }
                _ => return Err(Self::invalid("expected ',' or '}' in JSON object")),
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> io::Result<JsonValue> {
        self.offset += 1;
        self.skip_ws();
        let mut items = Vec::new();
        if self.bytes.get(self.offset) == Some(&b']') {
            self.offset += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            items.push(self.parse_value(depth)?);
            self.skip_ws();
            match self.bytes.get(self.offset) {
                Some(b',') => self.offset += 1,
                Some(b']') => {
                    self.offset += 1;
                    return Ok(JsonValue::Array(items));
                }
                _ => return Err(Self::invalid("expected ',' or ']' in JSON array")),
            }
        }
    }

    fn parse_hex4(&mut self) -> io::Result<u16> {
        let Some(slice) = self.bytes.get(self.offset..self.offset + 4) else {
            return Err(Self::invalid("truncated unicode escape in JSON string"));
        };
        if !slice.iter().all(u8::is_ascii_hexdigit) {
            return Err(Self::invalid("invalid unicode escape"));
        }
        let text =
            std::str::from_utf8(slice).map_err(|_| Self::invalid("invalid unicode escape"))?;
        let code =
            u16::from_str_radix(text, 16).map_err(|_| Self::invalid("invalid unicode escape"))?;
        self.offset += 4;
        Ok(code)
    }

    fn parse_string(&mut self) -> io::Result<String> {
        self.offset += 1;
        let mut out = Vec::new();
        loop {
            let Some(&byte) = self.bytes.get(self.offset) else {
                return Err(Self::invalid("unterminated JSON string"));
            };
            self.offset += 1;
            match byte {
                b'"' => {
                    return String::from_utf8(out)
                        .map_err(|_| Self::invalid("JSON string is not valid UTF-8"));
                }
                b'\\' => {
                    let Some(&esc) = self.bytes.get(self.offset) else {
                        return Err(Self::invalid("unterminated escape in JSON string"));
                    };
                    self.offset += 1;
                    match esc {
                        b'"' => out.push(b'"'),
                        b'\\' => out.push(b'\\'),
                        b'/' => out.push(b'/'),
                        b'b' => out.push(0x08),
                        b'f' => out.push(0x0c),
                        b'n' => out.push(b'\n'),
                        b'r' => out.push(b'\r'),
                        b't' => out.push(b'\t'),
                        b'u' => {
                            let first = self.parse_hex4()?;
                            let scalar = if (0xD800..=0xDBFF).contains(&first) {
                                if !self.bytes[self.offset..].starts_with(b"\\u") {
                                    return Err(Self::invalid("unpaired UTF-16 high surrogate"));
                                }
                                self.offset += 2;
                                let second = self.parse_hex4()?;
                                if !(0xDC00..=0xDFFF).contains(&second) {
                                    return Err(Self::invalid("invalid UTF-16 low surrogate"));
                                }
                                0x10000
                                    + ((u32::from(first - 0xD800) << 10)
                                        | u32::from(second - 0xDC00))
                            } else if (0xDC00..=0xDFFF).contains(&first) {
                                return Err(Self::invalid("unpaired UTF-16 low surrogate"));
                            } else {
                                u32::from(first)
                            };
                            let ch = char::from_u32(scalar)
                                .ok_or_else(|| Self::invalid("invalid unicode scalar"))?;
                            let mut buf = [0u8; 4];
                            out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                        }
                        _ => return Err(Self::invalid("unsupported JSON escape")),
                    }
                }
                0..=0x1f => {
                    return Err(Self::invalid("unescaped control character in JSON string"))
                }
                _ => out.push(byte),
            }
        }
    }

    fn parse_number(&mut self) -> io::Result<String> {
        let start = self.offset;
        if self.bytes.get(self.offset) == Some(&b'-') {
            self.offset += 1;
        }
        match self.bytes.get(self.offset) {
            Some(b'0') => self.offset += 1,
            Some(b'1'..=b'9') => {
                self.offset += 1;
                while matches!(self.bytes.get(self.offset), Some(b'0'..=b'9')) {
                    self.offset += 1;
                }
            }
            _ => return Err(Self::invalid("invalid JSON number")),
        }
        if self.bytes.get(self.offset) == Some(&b'.') {
            self.offset += 1;
            if !matches!(self.bytes.get(self.offset), Some(b'0'..=b'9')) {
                return Err(Self::invalid("invalid JSON fraction"));
            }
            while matches!(self.bytes.get(self.offset), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
        }
        if matches!(self.bytes.get(self.offset), Some(b'e' | b'E')) {
            self.offset += 1;
            if matches!(self.bytes.get(self.offset), Some(b'+' | b'-')) {
                self.offset += 1;
            }
            if !matches!(self.bytes.get(self.offset), Some(b'0'..=b'9')) {
                return Err(Self::invalid("invalid JSON exponent"));
            }
            while matches!(self.bytes.get(self.offset), Some(b'0'..=b'9')) {
                self.offset += 1;
            }
        }
        std::str::from_utf8(&self.bytes[start..self.offset])
            .map(str::to_owned)
            .map_err(|_| Self::invalid("invalid JSON number"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::{symlink, PermissionsExt};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn test_root(label: &str) -> std::path::PathBuf {
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let dir = std::env::temp_dir().join(format!(
            "codex-daemon-settings-{label}-{}-{nanos}-{seq}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn creates_default_settings_and_preserves_idempotency() {
        let root = test_root("create");
        let home = root.join("codex-home");
        ensure(&home).unwrap();
        let settings = home.join("app-server-daemon/settings.json");
        assert_eq!(
            fs::read(&settings).unwrap(),
            b"{\"updater\":{\"autoUpdateEnabled\":false}}\n"
        );
        ensure(&home).unwrap();
        assert_eq!(
            fs::read(&settings).unwrap(),
            b"{\"updater\":{\"autoUpdateEnabled\":false}}\n"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn merges_existing_settings_preserving_unrelated_keys() {
        let root = test_root("merge");
        let home = root.join("codex-home");
        let daemon = home.join("app-server-daemon");
        fs::create_dir_all(&daemon).unwrap();
        let settings = daemon.join("settings.json");
        fs::write(
            &settings,
            br#"{"custom":42,"updater":{"lastCheck":"2026-11-15","autoUpdateEnabled":true},"tags":["a\nb"]}"#,
        )
        .unwrap();

        ensure(&home).unwrap();
        assert_eq!(
            fs::read_to_string(&settings).unwrap(),
            "{\"custom\":42,\"updater\":{\"lastCheck\":\"2026-11-15\",\"autoUpdateEnabled\":false},\"tags\":[\"a\\nb\"]}\n"
        );
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn every_updater_flag_is_false_even_with_duplicate_escaped_keys() {
        let input = br#"{"updater":{"autoUpdateEnabled":true,"setting":5,"autoUpdateEnabled":false},"updat\u0065r":{"autoUpdateEnabled":true,"autoUpdateEnabl\u0065d":false},"updater":null}"#;
        let updated = update_settings_bytes(Some(input)).unwrap().unwrap();
        let text = std::str::from_utf8(&updated).unwrap();
        assert_eq!(text.matches(r#""autoUpdateEnabled":false"#).count(), 5);
        assert!(!text.contains("true"));
        assert!(text.contains(r#""setting":5"#));
        assert!(update_settings_bytes(Some(&updated)).unwrap().is_none());
    }

    #[test]
    fn json_unicode_escape_requires_four_hex_digits() {
        for input in [
            br#"{"key":"\u+041"}"#.as_slice(),
            br#"{"key":"\u00g1"}"#,
            br#"{"key":"\u 041"}"#,
        ] {
            assert_eq!(
                update_settings_bytes(Some(input)).unwrap_err().kind(),
                ErrorKind::InvalidData
            );
        }
        assert!(update_settings_bytes(Some(br#"{"key":"\u0041"}"#))
            .unwrap()
            .is_some());
    }

    #[test]
    fn rejects_symlink_settings_or_corrupt_json() {
        let root = test_root("reject");
        let home = root.join("codex-home");
        let daemon = home.join("app-server-daemon");
        fs::create_dir_all(&daemon).unwrap();
        let outside = root.join("outside.json");
        fs::write(&outside, b"{}").unwrap();
        symlink(&outside, daemon.join("settings.json")).unwrap();
        assert!(ensure(&home).is_err());

        fs::remove_file(daemon.join("settings.json")).unwrap();
        fs::write(daemon.join("settings.json"), b"not-json").unwrap();
        assert!(ensure(&home).is_err());
        fs::remove_dir_all(&root).unwrap();
    }

    fn make_standalone_fixture(root: &Path) -> (std::path::PathBuf, std::path::PathBuf) {
        let standalone = root.join(".codex/packages/standalone");
        let rel = standalone.join("releases/0.170.0-3f7e1a");
        fs::create_dir_all(rel.join("bin")).unwrap();
        fs::write(rel.join("bin/codex"), b"binary").unwrap();
        let current = standalone.join("current");
        symlink("releases/0.170.0-3f7e1a", &current).unwrap();
        (standalone, current)
    }

    #[test]
    fn creates_default_package_link_only_for_uninitialized_home() {
        let root = test_root("pkg-link-fresh");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);
        let codex_home = root.join("fresh-home");
        fs::create_dir_all(&codex_home).unwrap();
        fs::set_permissions(&codex_home, fs::Permissions::from_mode(0o755)).unwrap();

        ensure_default_package_link(&codex_home, &standalone_current, &standalone_root).unwrap();
        let link = codex_home.join("packages/app-server-daemon/current");
        assert!(fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink());
        assert_eq!(fs::read_link(&link).unwrap(), standalone_current);
        assert_eq!(
            fs::metadata(&codex_home).unwrap().permissions().mode() & 0o777,
            0o755
        );

        // Second call hits fast path without replacing or failing.
        ensure_default_package_link(&codex_home, &standalone_current, &standalone_root).unwrap();
        assert_eq!(fs::read_link(&link).unwrap(), standalone_current);
        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn preserves_existing_dedicated_broken_root_symlink_and_partial_daemon_state() {
        let root = test_root("pkg-link-preserve");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);

        // 1. Existing pinned/different symlink target is preserved.
        let h_pinned = root.join("h-pinned");
        let pinned_target = root.join("other-gen");
        fs::create_dir_all(pinned_target.join("bin")).unwrap();
        fs::create_dir_all(h_pinned.join("packages/app-server-daemon")).unwrap();
        symlink(
            &pinned_target,
            h_pinned.join("packages/app-server-daemon/current"),
        )
        .unwrap();
        ensure_default_package_link(&h_pinned, &standalone_current, &standalone_root).unwrap();
        assert_eq!(
            fs::read_link(h_pinned.join("packages/app-server-daemon/current")).unwrap(),
            pinned_target
        );

        // 2. Broken symlink is preserved rather than overwritten.
        let h_broken = root.join("h-broken");
        fs::create_dir_all(h_broken.join("packages/app-server-daemon")).unwrap();
        symlink(
            "/nonexistent/target",
            h_broken.join("packages/app-server-daemon/current"),
        )
        .unwrap();
        ensure_default_package_link(&h_broken, &standalone_current, &standalone_root).unwrap();
        assert_eq!(
            fs::read_link(h_broken.join("packages/app-server-daemon/current")).unwrap(),
            Path::new("/nonexistent/target")
        );

        // 3. Symlinked packages/app-server-daemon root is never traversed or mutated.
        let h_root_sym = root.join("h-root-sym");
        let external_store = root.join("external-store");
        fs::create_dir_all(&external_store).unwrap();
        fs::create_dir_all(h_root_sym.join("packages")).unwrap();
        symlink(
            &external_store,
            h_root_sym.join("packages/app-server-daemon"),
        )
        .unwrap();
        ensure_default_package_link(&h_root_sym, &standalone_current, &standalone_root).unwrap();
        assert!(!external_store.join("current").exists());

        // 4. Incomplete/partial package state (`releases/`, `install.lock`, `selection-history`) prevents auto-link.
        for indicator in [
            "releases",
            "install.lock",
            "selection-history",
            "auto-update-version",
        ] {
            let h_partial = root.join(format!("h-partial-{indicator}"));
            let pkg_dir = h_partial.join("packages/app-server-daemon");
            fs::create_dir_all(&pkg_dir).unwrap();
            if indicator == "releases" {
                fs::create_dir_all(pkg_dir.join(indicator)).unwrap();
            } else {
                fs::write(pkg_dir.join(indicator), b"").unwrap();
            }
            ensure_default_package_link(&h_partial, &standalone_current, &standalone_root).unwrap();
            assert!(!pkg_dir.join("current").exists());
        }

        // Modern and legacy launch traces each preserve upstream's selection,
        // even if only a stale log or dangling symlink remains.
        for trace in PRIOR_DAEMON_TRACE_FILES {
            let home = root.join(format!("h-{trace}"));
            let state = home.join("app-server-daemon");
            fs::create_dir_all(&state).unwrap();
            symlink("missing", state.join(trace)).unwrap();
            ensure_default_package_link(&home, &standalone_current, &standalone_root).unwrap();
            assert!(!home.join("packages").exists(), "{trace}");
        }

        let h_stopped_empty = root.join("h-stopped-empty");
        ensure(&h_stopped_empty).unwrap();
        fs::write(h_stopped_empty.join("app-server-daemon/daemon.lock"), b"").unwrap();
        ensure_default_package_link(&h_stopped_empty, &standalone_current, &standalone_root)
            .unwrap();
        assert_eq!(
            fs::read_link(h_stopped_empty.join("packages/app-server-daemon/current")).unwrap(),
            standalone_current
        );

        // 6. Never creates files inside standalone release root.
        let inside_release = standalone_root.join("releases/0.170.0-3f7e1a");
        ensure_default_package_link(&inside_release, &standalone_current, &standalone_root)
            .unwrap();
        assert!(!inside_release.join("packages").exists());

        fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn handles_concurrent_first_initialization_with_eexist() {
        let root = test_root("pkg-link-concurrent");
        let (standalone_root, standalone_current) = make_standalone_fixture(&root);
        let codex_home = root.join("shared-concurrent-home");
        fs::create_dir_all(&codex_home).unwrap();

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let mut handles = Vec::new();
        for _ in 0..4 {
            let b = barrier.clone();
            let h = codex_home.clone();
            let sc = standalone_current.clone();
            let sr = standalone_root.clone();
            handles.push(std::thread::spawn(move || {
                b.wait();
                ensure_default_package_link(&h, &sc, &sr).unwrap();
            }));
        }
        for handle in handles {
            handle.join().unwrap();
        }
        assert_eq!(
            fs::read_link(codex_home.join("packages/app-server-daemon/current")).unwrap(),
            standalone_current
        );
        fs::remove_dir_all(&root).unwrap();
    }
}
