use std::fs::{self, DirBuilder, File, OpenOptions};
use std::io::{self, ErrorKind, Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
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
    match entries.iter().rposition(|(key, _)| key == "updater") {
        None => {
            entries.push((
                "updater".to_owned(),
                JsonValue::Object(vec![(
                    "autoUpdateEnabled".to_owned(),
                    JsonValue::Bool(false),
                )]),
            ));
            changed = true;
        }
        Some(index) => match &mut entries[index].1 {
            JsonValue::Object(sub_entries) => {
                match sub_entries
                    .iter()
                    .rposition(|(key, _)| key == "autoUpdateEnabled")
                {
                    Some(flag_idx) if matches!(sub_entries[flag_idx].1, JsonValue::Bool(false)) => {
                    }
                    Some(flag_idx) => {
                        sub_entries[flag_idx].1 = JsonValue::Bool(false);
                        changed = true;
                    }
                    None => {
                        sub_entries.push(("autoUpdateEnabled".to_owned(), JsonValue::Bool(false)));
                        changed = true;
                    }
                }
            }
            JsonValue::Null => {
                entries[index].1 = JsonValue::Object(vec![(
                    "autoUpdateEnabled".to_owned(),
                    JsonValue::Bool(false),
                )]);
                changed = true;
            }
            _ => {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    "app-server-daemon settings.json updater field is not an object",
                ));
            }
        },
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
    use std::os::unix::fs::symlink;
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
}
