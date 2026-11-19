//! Bounded JSON with insertion-ordered object fields and preserved number spelling.
use std::io::{self, ErrorKind};

const MAX_JSON_DEPTH: usize = 32;

#[derive(Debug, Clone, PartialEq)]
pub(super) enum JsonValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl JsonValue {
    pub(super) fn write_json(&self, out: &mut String) {
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
    out.push('"');
    escape_into(text, out, |ch| ch < '\u{20}');
    out.push('"');
}

pub(super) fn json_escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    escape_into(value, &mut escaped, char::is_control);
    escaped
}

fn escape_into(text: &str, out: &mut String, is_control: impl Fn(char) -> bool) {
    use std::fmt::Write as _;

    for ch in text.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\u{0C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            ch if is_control(ch) => {
                let _ = write!(out, "\\u{:04x}", ch as u32);
            }
            ch => out.push(ch),
        }
    }
}

pub(super) struct JsonParser<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> JsonParser<'a> {
    pub(super) fn new(bytes: &'a [u8]) -> Self {
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

    pub(super) fn parse_document(&mut self) -> io::Result<JsonValue> {
        let value = self.parse_value(MAX_JSON_DEPTH - 1, true)?;
        self.skip_ws();
        if self.offset != self.bytes.len() {
            return Err(Self::invalid("trailing data after JSON document"));
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth_left: usize, retain: bool) -> io::Result<JsonValue> {
        self.skip_ws();
        let Some(&byte) = self.bytes.get(self.offset) else {
            return Err(Self::invalid("unexpected end of JSON"));
        };
        match byte {
            b'{' => self.parse_object(depth_left, retain),
            b'[' => self.parse_array(depth_left, retain),
            b'"' => self.parse_string().map(JsonValue::String),
            b't' => self.expect_literal(b"true", JsonValue::Bool(true)),
            b'f' => self.expect_literal(b"false", JsonValue::Bool(false)),
            b'n' => self.expect_literal(b"null", JsonValue::Null),
            b'-' | b'0'..=b'9' => {
                let raw = self.parse_number()?;
                Ok(if retain {
                    JsonValue::Number(raw.to_owned())
                } else {
                    JsonValue::Null
                })
            }
            _ => Err(Self::invalid("unexpected token in JSON")),
        }
    }

    pub(super) fn skip_value(&mut self) -> io::Result<()> {
        // Ignored metadata fields still receive full syntax checking.
        self.parse_value(MAX_JSON_DEPTH, false).map(drop)
    }

    pub(super) fn peek(&mut self) -> Option<u8> {
        self.skip_ws();
        self.bytes.get(self.offset).copied()
    }

    pub(super) fn consume(&mut self, byte: u8) -> bool {
        if self.peek() == Some(byte) {
            self.offset += 1;
            true
        } else {
            false
        }
    }

    pub(super) fn expect(&mut self, byte: u8) -> io::Result<()> {
        if self.consume(byte) {
            Ok(())
        } else {
            Err(Self::invalid("unexpected token in JSON"))
        }
    }

    pub(super) fn string(&mut self) -> io::Result<String> {
        if self.peek() == Some(b'"') {
            self.parse_string()
        } else {
            Err(Self::invalid("expected JSON string"))
        }
    }

    pub(super) fn literal(&mut self, literal: &[u8]) -> io::Result<()> {
        self.skip_ws();
        self.read_literal(literal)
    }

    fn expect_literal(&mut self, literal: &[u8], value: JsonValue) -> io::Result<JsonValue> {
        self.read_literal(literal)?;
        Ok(value)
    }

    fn read_literal(&mut self, literal: &[u8]) -> io::Result<()> {
        if self.bytes[self.offset..].starts_with(literal) {
            self.offset += literal.len();
            Ok(())
        } else {
            Err(Self::invalid("invalid JSON literal"))
        }
    }

    fn parse_object(&mut self, depth_left: usize, retain: bool) -> io::Result<JsonValue> {
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
            let value = self.parse_value(child_depth(depth_left)?, retain)?;
            if retain {
                entries.push((key, value));
            }
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

    fn parse_array(&mut self, depth_left: usize, retain: bool) -> io::Result<JsonValue> {
        self.offset += 1;
        self.skip_ws();
        let mut items = Vec::new();
        if self.bytes.get(self.offset) == Some(&b']') {
            self.offset += 1;
            return Ok(JsonValue::Array(items));
        }
        loop {
            let value = self.parse_value(child_depth(depth_left)?, retain)?;
            if retain {
                items.push(value);
            }
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

    fn parse_number(&mut self) -> io::Result<&str> {
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
            .map_err(|_| Self::invalid("invalid JSON number"))
    }
}
fn child_depth(depth_left: usize) -> io::Result<usize> {
    depth_left
        .checked_sub(1)
        .ok_or_else(|| JsonParser::invalid("JSON nesting exceeds maximum depth"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unicode_escapes_require_four_hex_digits_and_paired_surrogates() {
        let mut parser = JsonParser::new(br#""\u0041\uD83D\uDE80""#);
        assert_eq!(parser.string().unwrap(), "A🚀");
        for raw in [
            br#""\u+041""#.as_slice(),
            br#""\u041""#,
            br#""\uDE80""#,
            br#""\uD83D\u0041""#,
            br#""\uD83D""#,
        ] {
            assert_eq!(
                JsonParser::new(raw).parse_document().unwrap_err().kind(),
                ErrorKind::InvalidData
            );
            assert!(JsonParser::new(raw).skip_value().is_err());
        }
    }

    #[test]
    fn nesting_limits_preserve_the_document_and_metadata_boundaries() {
        let nested = |depth| format!("{}null{}", "[".repeat(depth), "]".repeat(depth));
        let within = nested(MAX_JSON_DEPTH - 1);
        let boundary = nested(MAX_JSON_DEPTH);
        let beyond = nested(MAX_JSON_DEPTH + 1);

        assert!(JsonParser::new(within.as_bytes()).parse_document().is_ok());
        assert!(JsonParser::new(boundary.as_bytes())
            .parse_document()
            .is_err());
        assert!(JsonParser::new(boundary.as_bytes()).skip_value().is_ok());
        assert!(JsonParser::new(beyond.as_bytes()).skip_value().is_err());
    }

    #[test]
    fn serializer_preserves_object_order_number_spelling_and_escape_conventions() {
        let raw =
            r#"{"z":-0.0E+2,"a":"\uD83D\uDE80","z":[true,false,null],"control":"\u007f\u0001"}"#;
        let value = JsonParser::new(raw.as_bytes()).parse_document().unwrap();
        let mut serialized = String::new();
        value.write_json(&mut serialized);
        assert_eq!(
            serialized,
            "{\"z\":-0.0E+2,\"a\":\"🚀\",\"z\":[true,false,null],\"control\":\"\u{7f}\\u0001\"}"
        );
        assert_eq!(json_escape("\u{7f}\u{1}"), r"\u007f\u0001");
    }
}
