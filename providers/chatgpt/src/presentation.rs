//! Source-aware, static presentation projection. No executable UI or JS runtime.
#[path = "presentation/render.rs"]
mod render;

#[derive(Clone, Debug, PartialEq)]
enum Value {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
    Unknown(String),
}

#[derive(Clone, Debug, PartialEq)]
enum Node {
    Text(String),
    Element {
        name: String,
        attrs: Vec<(String, Value)>,
        children: Vec<Node>,
    },
}

fn append_nodes(out: &mut Vec<Node>, nodes: Vec<Node>) {
    for node in nodes {
        match (out.last_mut(), node) {
            (Some(Node::Text(prev)), Node::Text(next)) => prev.push_str(&next),
            (_, node) => out.push(node),
        }
    }
}

type Locals = Vec<(String, Value)>;

pub fn render_for_codex_tui(input: &str) -> String {
    project(input, false)
}
pub fn render_stream_prefix(input: &str) -> String {
    project(input, true)
}

fn project(input: &str, streaming: bool) -> String {
    let mut parser = Parser {
        source: input,
        pos: 0,
        locals: Vec::new(),
    };
    let mut out = String::with_capacity(input.len());
    while parser.pos < input.len() {
        let start = parser.pos;
        match parser.unit(0, Context::Prose) {
            Some(nodes) => {
                for node in nodes {
                    match node {
                        Node::Text(text) => out.push_str(&text),
                        node => out.push_str(&render::render_node(&node)),
                    }
                }
            }
            None => {
                if !streaming {
                    out.push_str(&input[start..]);
                }
                break;
            }
        }
    }
    out
}

const TAGS: &[&str] = &[
    "svg",
    "grid",
    "grid-item",
    "row",
    "col",
    "card",
    "box",
    "badge",
    "title",
    "caption",
    "text",
    "Link",
    "MemoryCite",
    "button",
    "checkbox",
    "list",
    "list-item",
    "divider",
    "spacer",
    "icon",
    "slider",
    "table",
    "table-section",
    "table-row",
    "table-cell",
    "Chart",
    "segmented-control",
    "radio-group",
    "radio",
    "select",
    "input",
    "textarea",
    "form",
    "date-picker",
    "popover",
    "popover-trigger",
    "popover-content",
    "carousel",
    "carousel-item",
    "carousel-media-item",
    "flow",
    "flow-item",
    "pressable",
    "math",
    "bold",
    "italic",
    "underline",
    "strikethrough",
    "code",
    "label",
    "blockquote",
    "AsyncImage",
    "AsyncImageGroup",
    "AsyncVideo",
    "MapWidgetV2",
    "Entity",
    "Cite",
    "FileCite",
    "FileNavList",
    "FollowUp",
    "AppBlock",
    "CodeBlock",
    "WritingBlock",
    "escape",
    "favicon",
];

fn ident(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '-')
}
fn name_len(s: &str) -> usize {
    s.bytes().take_while(|b| ident(*b as char)).count()
}

// Quotes and nested delimiters are shared by attributes, literals and directives.
fn balanced(s: &str) -> Option<usize> {
    let mut stack = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    for (i, c) in s.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == q {
                quote = None;
            }
            continue;
        }
        match c {
            '\'' | '"' | '`' => quote = Some(c),
            '{' => stack.push('}'),
            '[' => stack.push(']'),
            '(' => stack.push(')'),
            '}' | ']' | ')' => {
                if stack.pop() != Some(c) {
                    return None;
                }
                if stack.is_empty() {
                    return Some(i + c.len_utf8());
                }
            }
            _ => {}
        }
    }
    None
}

fn quoted(s: &str) -> Option<(usize, String)> {
    let q = s.chars().next()?;
    if !matches!(q, '"' | '\'') {
        return None;
    }
    let mut out = String::new();
    let mut chars = s[q.len_utf8()..].char_indices();
    while let Some((i, c)) = chars.next() {
        if c == q {
            return Some((i + 2, out));
        }
        if c == '\\' {
            let (_, c) = chars.next()?;
            match c {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                '\\' | '"' | '\'' => out.push(c),
                _ => {
                    out.push('\\');
                    out.push(c);
                }
            }
        } else {
            out.push(c);
        }
    }
    None
}

fn split_top(s: &str, delimiter: char) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        let skip = match c {
            '"' | '\'' => quoted(rest).map(|(n, _)| n),
            '{' | '[' | '(' => balanced(rest),
            _ => None,
        };
        if let Some(n) = skip {
            i += n;
            continue;
        }
        if c == delimiter {
            parts.push(&s[start..i]);
            start = i + c.len_utf8();
        }
        i += c.len_utf8();
    }
    parts.push(&s[start..]);
    parts
}

fn eval(s: &str, locals: &Locals, depth: usize) -> Value {
    let s = s.trim();
    let unknown = || Value::Unknown(s.to_owned());
    // Stack safety: never discard the original expression on fallback.
    if depth >= 128 {
        return unknown();
    }
    match s {
        "null" => return Value::Null,
        "true" => return Value::Bool(true),
        "false" => return Value::Bool(false),
        _ => {}
    }
    if let Some((n, v)) = quoted(s) {
        if n == s.len() {
            return Value::String(v);
        }
    }
    if !s.is_empty() && s.parse::<f64>().is_ok_and(f64::is_finite) {
        return Value::Number(s.to_owned());
    }
    if s.starts_with('[') && balanced(s) == Some(s.len()) {
        return Value::Array(
            split_top(&s[1..s.len() - 1], ',')
                .into_iter()
                .filter(|p| !p.trim().is_empty())
                .map(|p| eval(p, locals, depth + 1))
                .collect(),
        );
    }
    if s.starts_with('{') && balanced(s) == Some(s.len()) {
        let mut fields = Vec::new();
        for p in split_top(&s[1..s.len() - 1], ',') {
            if p.trim().is_empty() {
                continue;
            }
            let pair = split_top(p, ':');
            if pair.len() != 2 {
                return unknown();
            }
            let key = pair[0].trim();
            let key = quoted(key)
                .filter(|(n, _)| *n == key.len())
                .map(|(_, v)| v)
                .or_else(|| (!key.is_empty() && key.chars().all(ident)).then(|| key.to_owned()));
            let Some(key) = key else {
                return unknown();
            };
            fields.push((key, eval(pair[1], locals, depth + 1)));
        }
        return Value::Object(fields);
    }
    if s.starts_with('(') && balanced(s) == Some(s.len()) {
        return eval(&s[1..s.len() - 1], locals, depth + 1);
    }
    if let Some(rhs) = s.strip_prefix('!') {
        if let Value::Bool(b) = eval(rhs, locals, depth + 1) {
            return Value::Bool(!b);
        }
    }
    // Only a single unambiguous top-level operation is interpreted. Do not
    // approximate JS coercion, precedence, associativity or object identity.
    let mut operation = None;
    let mut i = 0;
    while i < s.len() {
        let rest = &s[i..];
        let c = rest.chars().next().unwrap();
        let skip = match c {
            '"' | '\'' => quoted(rest).map(|(n, _)| n),
            '{' | '[' | '(' => balanced(rest),
            _ => None,
        };
        if let Some(n) = skip {
            i += n;
            continue;
        }
        if let Some(op) = [" === ", " !== ", " + ", " - ", " * ", " / "]
            .into_iter()
            .find(|op| rest.starts_with(op))
        {
            if operation.replace((i, op)).is_some() {
                return unknown();
            }
            i += op.len();
        } else {
            i += c.len_utf8();
        }
    }
    if let Some((i, op)) = operation {
        let a = eval(&s[..i], locals, depth + 1);
        let b = eval(&s[i + op.len()..], locals, depth + 1);
        if matches!(op, " === " | " !== ") {
            let equal = match (&a, &b) {
                (Value::Unknown(_) | Value::Array(_) | Value::Object(_), _)
                | (_, Value::Unknown(_) | Value::Array(_) | Value::Object(_)) => return unknown(),
                (Value::Number(a), Value::Number(b)) => {
                    a.parse::<f64>().ok() == b.parse::<f64>().ok()
                }
                _ => a == b,
            };
            return Value::Bool(equal == (op == " === "));
        }
        if let (Value::Number(a), Value::Number(b)) = (a, b) {
            if let (Ok(a), Ok(b)) = (a.parse::<f64>(), b.parse::<f64>()) {
                let n = match op {
                    " + " => a + b,
                    " - " => a - b,
                    " * " => a * b,
                    _ => a / b,
                };
                if n.is_finite() {
                    return Value::Number(n.to_string());
                }
            }
        }
        return unknown();
    }
    let n = name_len(s);
    if let Some((_, value)) = locals.iter().rev().find(|(key, _)| key == &s[..n]) {
        let mut value = value.clone();
        let mut tail = &s[n..];
        while !tail.is_empty() {
            if let Some(rest) = tail.strip_prefix('.') {
                let n = name_len(rest);
                value = match value {
                    Value::Object(fields) => fields
                        .into_iter()
                        .find(|(k, _)| k == &rest[..n])
                        .map(|(_, v)| v)
                        .unwrap_or_else(unknown),
                    Value::Array(items) if &rest[..n] == "length" => {
                        Value::Number(items.len().to_string())
                    }
                    _ => return unknown(),
                };
                tail = &rest[n..];
            } else if tail.starts_with('[') {
                let Some(n) = balanced(tail) else {
                    return unknown();
                };
                value = match (value, tail[1..n - 1].trim().parse::<usize>()) {
                    (Value::Array(items), Ok(index)) => {
                        items.get(index).cloned().unwrap_or_else(unknown)
                    }
                    _ => return unknown(),
                };
                tail = &tail[n..];
            } else {
                return unknown();
            }
        }
        return value;
    }
    unknown()
}

fn value_text(v: Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) | Value::String(v) => v
            .chars()
            .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
            .collect(),
        Value::Unknown(v) => format!("[Unavailable: {v}]"),
        Value::Array(v) => format!(
            "[{}]",
            v.into_iter().map(value_text).collect::<Vec<_>>().join(", ")
        ),
        Value::Object(v) => format!(
            "{{{}}}",
            v.into_iter()
                .map(|(k, v)| format!("{k}: {}", value_text(v)))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn escaped_at(s: &str, i: usize) -> bool {
    s.as_bytes()[..i]
        .iter()
        .rev()
        .take_while(|&&b| b == b'\\')
        .count()
        % 2
        != 0
}

// The opening dollar can be plain currency or math. Do not project the
// ambiguous region until a matching delimiter or a line break resolves it.
fn dollar_span(s: &str) -> Option<usize> {
    let run = s.bytes().take_while(|&b| b == b'$').count();
    if run > 2 {
        return Some(run);
    }
    if s.len() == run {
        return None;
    }
    if run == 1 && s[run..].starts_with(char::is_whitespace) {
        return Some(run);
    }
    let mut i = run;
    while i < s.len() {
        let c = s[i..].chars().next()?;
        if run == 1 && c == '\n' {
            return Some(run);
        }
        if c == '$' && !escaped_at(s, i) {
            let close = s[i..].bytes().take_while(|&b| b == b'$').count();
            if close == run && i + close < s.len() {
                return Some(i + close);
            }
            i += close;
        } else {
            i += c.len_utf8();
        }
    }
    None
}

fn tag_end(s: &str) -> Option<usize> {
    let mut i = 1;
    while i < s.len() {
        let r = &s[i..];
        let c = r.chars().next()?;
        match c {
            '>' => return Some(i + 1),
            '"' | '\'' => i += quoted(r)?.0,
            '{' => i += balanced(r)?,
            _ => i += c.len_utf8(),
        }
    }
    None
}

// A literal XML region is opaque to presentation parsing, including its children.
fn literal_element_end(s: &str, name: &str, opening: usize) -> Option<usize> {
    let mut pos = opening;
    let mut depth = 1usize;
    while let Some(offset) = s[pos..].find('<') {
        pos += offset;
        let rest = &s[pos..];
        if rest.starts_with("<!--") {
            pos += rest.find("-->")? + 3;
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            pos += rest.find("]]>")? + 3;
            continue;
        }
        let closing = rest.starts_with("</");
        let prefix = if closing { 2 } else { 1 };
        let tail = &rest[prefix..];
        let n = tag_name_len(tail);
        if n == 0 || !tag_boundary(&tail[n..]) {
            pos += 1;
            continue;
        }
        let end = tag_end(rest)?;
        if &tail[..n] == name {
            if closing {
                depth -= 1;
                if depth == 0 {
                    return Some(pos + end);
                }
            } else if !rest[..end - 1].trim_end().ends_with('/') {
                depth += 1;
            }
        }
        pos += end;
    }
    None
}

fn tag_name_len(s: &str) -> usize {
    s.bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b':' | b'.'))
        .count()
}

fn tag_boundary(s: &str) -> bool {
    s.starts_with(|c: char| c.is_whitespace() || matches!(c, '/' | '>'))
}

fn html_void(name: &str) -> bool {
    matches!(
        name,
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}

fn svg_tag(name: &str) -> bool {
    matches!(
        name,
        "g" | "rect"
            | "circle"
            | "ellipse"
            | "line"
            | "path"
            | "polyline"
            | "polygon"
            | "tspan"
            | "desc"
            | "defs"
            | "marker"
            | "symbol"
            | "use"
            | "clipPath"
            | "mask"
            | "linearGradient"
            | "radialGradient"
            | "stop"
            | "image"
    )
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Context {
    Prose,
    Element,
    Svg,
}

fn attrs(mut s: &str, locals: &Locals) -> Vec<(String, Value)> {
    let mut result = Vec::new();
    loop {
        s = s.trim_start();
        if s.is_empty() {
            break;
        }
        let n = name_len(s);
        if n == 0 {
            result.push(("unavailable".into(), Value::Unknown(s.into())));
            break;
        }
        let key = &s[..n];
        s = s[n..].trim_start();
        let value;
        if let Some(rest) = s.strip_prefix('=') {
            s = rest.trim_start();
            if let Some((n, v)) = quoted(s) {
                value = Value::String(v);
                s = &s[n..];
            } else if s.starts_with('{') {
                let Some(n) = balanced(s) else {
                    result.push((key.into(), Value::Unknown(s.into())));
                    break;
                };
                value = eval(&s[1..n - 1], locals, 0);
                s = &s[n..];
            } else {
                let n = s.find(char::is_whitespace).unwrap_or(s.len());
                value = Value::String(s[..n].into());
                s = &s[n..];
            }
        } else {
            value = Value::Bool(true);
        }
        result.push((key.into(), value));
    }
    result
}

struct Parser<'a> {
    source: &'a str,
    pos: usize,
    locals: Locals,
}

impl<'a> Parser<'a> {
    fn rest(&self) -> &'a str {
        &self.source[self.pos..]
    }
    fn text(&mut self, n: usize) -> Vec<Node> {
        let text = self.rest()[..n].to_owned();
        self.pos += n;
        vec![Node::Text(text)]
    }
    fn unit(&mut self, depth: usize, context: Context) -> Option<Vec<Node>> {
        let s = self.rest();
        if s.is_empty() {
            return None;
        }
        // At line boundaries, recognize Markdown container prefixes before fences.
        if self.pos == 0 || self.source.as_bytes()[self.pos - 1] == b'\n' {
            let line = s.split_inclusive('\n').next()?;
            let body = markdown_body(line);
            let trimmed = body.trim_start_matches(' ');
            let indent = body.len() - trimmed.len();
            let c = trimmed.chars().next().unwrap_or(' ');
            let n = trimmed.chars().take_while(|x| *x == c).count();
            if matches!(c, '`' | '~') && n >= 3 {
                // A fence opener must settle before its contents are released.
                if !line.ends_with('\n') {
                    return None;
                }
                let mut end = line.len();
                for l in s[line.len()..].split_inclusive('\n') {
                    end += l.len();
                    let b = markdown_body(l).trim_start_matches(' ');
                    let run = b.chars().take_while(|x| *x == c).count();
                    if run >= n && b[run..].trim().is_empty() {
                        if !l.ends_with('\n') {
                            return None;
                        }
                        return Some(self.text(end));
                    }
                }
                return None;
            }
            if context != Context::Prose && trimmed.starts_with('|') {
                if let Some((len, table)) = layout_pipe_table(s) {
                    self.pos += len;
                    return Some(vec![Node::Text(table)]);
                }
            }
            if indent >= 4 || body.starts_with('\t') {
                let tag = trimmed.strip_prefix('<').and_then(|t| {
                    let n = tag_name_len(t);
                    (n > 0 && tag_boundary(&t[n..])).then_some(&t[..n])
                });
                let explicit_layout = context != Context::Prose
                    && tag.is_some_and(|tag| {
                        TAGS.contains(&tag) || context == Context::Svg && svg_tag(tag)
                    });
                if !explicit_layout {
                    return Some(self.text(line.len()));
                }
            }
            // A partial container prefix might become a fence.
            if !line.ends_with('\n')
                && (trimmed.is_empty()
                    || trimmed.chars().all(|x| {
                        matches!(x, '`' | '~' | '>' | '-' | '+' | '*' | ' ' | '.' | '0'..='9')
                    }))
            {
                return None;
            }
        }
        let c = s.chars().next()?;
        if c == '$' {
            return Some(self.text(dollar_span(s)?));
        }
        if c == '\\' {
            for (open, close) in [("\\(", "\\)"), ("\\[", "\\]")] {
                if let Some(rest) = s.strip_prefix(open) {
                    let end = rest
                        .match_indices(close)
                        .map(|(i, _)| i + open.len())
                        .find(|&i| !escaped_at(s, i))?;
                    return Some(self.text(end + close.len()));
                }
            }
            let n = c.len_utf8() + s[c.len_utf8()..].chars().next()?.len_utf8();
            return Some(self.text(n));
        }
        if c == '`' {
            let n = s.bytes().take_while(|b| *b == b'`').count();
            let mut i = n;
            while i < s.len() {
                if s.as_bytes()[i] == b'`' {
                    let run = s[i..].bytes().take_while(|b| *b == b'`').count();
                    if run == n && i + run < s.len() {
                        return Some(self.text(i + run));
                    }
                    i += run;
                } else {
                    i += s[i..].chars().next()?.len_utf8();
                }
            }
            return None;
        }
        if c == '<' {
            for (open, close) in [("<!--", "-->"), ("<![CDATA[", "]]>")] {
                if open.starts_with(s) && s.len() < open.len() {
                    return None;
                }
                if let Some(rest) = s.strip_prefix(open) {
                    let end = rest.find(close)? + open.len() + close.len();
                    return Some(self.text(end));
                }
            }
            if s.starts_with("<!") || s.starts_with("<?") {
                return Some(self.text(tag_end(s)?));
            }
            let closing = s.starts_with("</");
            let prefix = if closing { 2 } else { 1 };
            if s.len() == prefix {
                return None;
            }
            let tail = &s[prefix..];
            let n = tag_name_len(tail);
            if n > 0 && tail.starts_with(|x: char| x.is_ascii_alphabetic()) {
                let name = &tail[..n];
                let after = &tail[n..];
                if after.is_empty() {
                    return None;
                }
                if tag_boundary(after) {
                    if !closing
                        && (TAGS.contains(&name) || context == Context::Svg && svg_tag(name))
                    {
                        return self.element(name, n, depth, context);
                    }
                    let end = tag_end(s)?;
                    let adjacent = self.source[..self.pos]
                        .chars()
                        .next_back()
                        .is_some_and(|ch| {
                            ch.is_alphanumeric() || matches!(ch, '_' | '>' | ']' | ')')
                        });
                    let inner = &s[prefix..end - 1];
                    let autolink = !inner.contains(char::is_whitespace)
                        && (inner.contains("://") || inner.contains('@'));
                    if closing
                        || adjacent
                        || autolink
                        || html_void(name)
                        || s[..end - 1].trim_end().ends_with('/')
                    {
                        return Some(self.text(end));
                    }
                    return Some(self.text(literal_element_end(s, name, end)?));
                }
                if after.starts_with('@') {
                    return Some(self.text(s.find('>')? + 1));
                }
            }
        }
        if c == '{' {
            let n = balanced(s)?;
            let expr = s[1..n - 1].trim();
            if let Some(condition) = expr.strip_prefix("#if ") {
                return self.condition(n, condition, depth, context);
            }
            if let Some(body) = expr
                .strip_prefix("@body ")
                .or_else(|| expr.strip_prefix("@module "))
            {
                self.pos += n;
                if let Some(decl) = body
                    .strip_prefix("const ")
                    .or_else(|| body.strip_prefix("let "))
                {
                    if let Some((key, val)) = decl.split_once('=') {
                        let key = key.trim();
                        let val = val.trim().trim_end_matches(';');
                        if !key.is_empty() && key.chars().all(ident) {
                            self.locals.push((key.into(), eval(val, &self.locals, 0)));
                            return Some(vec![]);
                        }
                        if let (Some(keys), Some(initial)) = (
                            key.strip_prefix('[').and_then(|k| k.strip_suffix(']')),
                            val.strip_prefix("DIL.useState(")
                                .and_then(|v| v.strip_suffix(')')),
                        ) {
                            if let Some(k) = keys.split(',').next() {
                                self.locals
                                    .push((k.trim().into(), eval(initial, &self.locals, 0)));
                                return Some(vec![]);
                            }
                        }
                    }
                }
                return Some(vec![Node::Text(format!(
                    "[Unavailable declaration: {body}]"
                ))]);
            }
            let v = eval(expr, &self.locals, 0);
            let root = name_len(expr);
            let local = root > 0 && self.locals.iter().any(|(key, _)| key == &expr[..root]);
            let member = root > 0
                && expr.starts_with(|ch: char| ch.is_ascii_alphabetic() || ch == '_')
                && expr[root..].starts_with('.')
                && !expr.contains([':', '\n']);
            if context != Context::Prose
                || local
                || member
                || expr.starts_with(['#', ':', '/', '@'])
            {
                self.pos += n;
                return Some(vec![Node::Text(value_text(v))]);
            }
            return Some(self.text(n));
        }
        let len = s
            .char_indices()
            .skip(1)
            .find(|&(i, ch)| {
                matches!(ch, '<' | '{' | '`' | '\\' | '$') || s.as_bytes()[i - 1] == b'\n'
            })
            .map_or(s.len(), |(i, _)| i);
        Some(self.text(len))
    }

    fn element(
        &mut self,
        name: &str,
        n: usize,
        depth: usize,
        context: Context,
    ) -> Option<Vec<Node>> {
        let s = self.rest();
        let end = tag_end(s)?;
        let raw = s[1 + n..end - 1].trim();
        let closed = raw.ends_with('/');
        let attributes = attrs(raw.strip_suffix('/').unwrap_or(raw), &self.locals);
        self.pos += end;
        let mut children = Vec::new();
        if !closed && !matches!(name, "divider" | "spacer" | "icon" | "slider") {
            let close = format!("</{name}>");
            if depth >= 128 {
                let finish = literal_element_end(s, name, end)?;
                self.pos += finish - end;
                return Some(vec![Node::Text(format!(
                    "[Presentation unavailable: nesting]\n{}",
                    &s[..finish]
                ))]);
            }
            let child_context = if name == "svg" || context == Context::Svg {
                Context::Svg
            } else {
                Context::Element
            };
            if matches!(name, "escape" | "AppBlock" | "CodeBlock" | "WritingBlock") {
                let n = self.rest().find(&close)?;
                children = self.text(n);
                self.pos += close.len();
            } else {
                loop {
                    if self.rest().starts_with(&close) {
                        self.pos += close.len();
                        break;
                    }
                    append_nodes(&mut children, self.unit(depth + 1, child_context)?);
                }
            }
        }
        if name == "escape" {
            return Some(children);
        }
        Some(vec![Node::Element {
            name: name.into(),
            attrs: attributes,
            children,
        }])
    }

    fn condition(
        &mut self,
        n: usize,
        condition: &str,
        depth: usize,
        context: Context,
    ) -> Option<Vec<Node>> {
        if depth >= 128 {
            return None;
        }
        self.pos += n;
        let saved = self.locals.clone();
        let mut branches = vec![(Some(condition), Vec::new())];
        loop {
            if self.rest().starts_with("{/if}") {
                self.pos += 5;
                break;
            }
            if self.rest().starts_with("{:else") {
                let end = balanced(self.rest())?;
                let directive = &self.rest()[1..end - 1];
                let next = if directive == ":else" {
                    Some(None)
                } else {
                    directive.strip_prefix(":else if ").map(Some)
                };
                if let Some(next) = next {
                    self.pos += end;
                    self.locals = saved.clone();
                    branches.push((next, Vec::new()));
                    continue;
                }
            }
            let nodes = self.unit(depth + 1, context)?;
            append_nodes(&mut branches.last_mut()?.1, nodes);
        }
        self.locals = saved;
        let mut result = Vec::new();
        let mut ambiguous = false;
        for (condition, nodes) in branches {
            let choice = condition.map_or(Value::Bool(true), |c| eval(c, &self.locals, 0));
            match choice {
                Value::Bool(false) => {}
                Value::Bool(true) => {
                    if ambiguous {
                        let label = condition.unwrap_or("else");
                        result.push(Node::Text(format!("\n[Alternative: {label}]\n")));
                    }
                    append_nodes(&mut result, nodes);
                    break;
                }
                _ => {
                    ambiguous = true;
                    let label = condition.unwrap_or("else");
                    result.push(Node::Text(format!("\n[Unknown condition: {label}; if]\n")));
                    append_nodes(&mut result, nodes);
                }
            }
        }
        Some(result)
    }
}

// A padded GFM table inside an explicit layout is not an indented code block.
// This is recognized only at the source boundary, never inside a protected
// code/math/markup region or by rewriting the already-rendered Markdown.
fn layout_pipe_table(s: &str) -> Option<(usize, String)> {
    let mut lines = s.split_inclusive('\n');
    let first = lines.next()?;
    let second = lines.next()?;
    let header = first.trim_start_matches(' ');
    let indent = first.len() - header.len();
    let delimiter = second.trim_start_matches(' ');
    if indent == 0
        || !header.starts_with('|')
        || !delimiter.starts_with('|')
        || second.len() - delimiter.len() != indent
        || !delimiter.trim().trim_matches('|').split('|').all(|c| {
            let c = c.trim().trim_matches(':');
            c.len() >= 3 && c.bytes().all(|b| b == b'-')
        })
    {
        return None;
    }
    let mut consumed = first.len() + second.len();
    let mut out = format!("{header}{delimiter}");
    for line in lines {
        let row = line.trim_start_matches(' ');
        if line.len() - row.len() != indent || !row.starts_with('|') {
            break;
        }
        consumed += line.len();
        out.push_str(row);
    }
    Some((consumed, out))
}

fn markdown_body(mut line: &str) -> &str {
    loop {
        let trimmed = line.trim_start_matches(' ');
        if line.len() - trimmed.len() >= 4 || line.starts_with('\t') {
            return line;
        }
        if let Some(rest) = trimmed.strip_prefix('>') {
            line = rest.strip_prefix(' ').unwrap_or(rest);
        } else if let Some(rest) = trimmed
            .strip_prefix("- ")
            .or_else(|| trimmed.strip_prefix("* "))
            .or_else(|| trimmed.strip_prefix("+ "))
        {
            line = rest;
        } else {
            let n = trimmed.bytes().take_while(u8::is_ascii_digit).count();
            if n > 0 {
                if let Some(rest) = trimmed[n..]
                    .strip_prefix(". ")
                    .or_else(|| trimmed[n..].strip_prefix(") "))
                {
                    line = rest;
                    continue;
                }
            }
            return line;
        }
    }
}

#[cfg(test)]
mod parser_tests {
    use super::*;
    #[test]
    fn protected_and_monotonic() {
        for s in [
            "Use `Vec<T>` and continue.",
            "x < 5. {name} done",
            "> Example:\n> ```html\n> <badge>literal</badge>\n> ```\nAfter.",
            "Example:\n\n    <badge>literal</badge>\n    {obj.value}\n",
            "Unicode 한국어 ✅ \\<badge>literal",
            "{#if true}Ready{:else}Not ready{/if} done",
            "{#if ready}Ready{:else}Not ready{/if} done",
        ] {
            let full = render_for_codex_tui(s);
            let mut previous = String::new();
            for i in s
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(s.len()))
            {
                let next = render_stream_prefix(&s[..i]);
                assert!(
                    next.starts_with(&previous) && full.starts_with(&next),
                    "{s:?} at {i}: {previous:?} -> {next:?}, full={full:?}"
                );
                previous = next;
            }
        }
    }
    #[test]
    fn branches_and_literals_are_not_guessed() {
        let raw = "{#if false}No{:else if true}Yes{:else}Never{/if}";
        assert_eq!(render_for_codex_tui(raw), "Yes");
        let raw = "{#if unknown}First{:else if true}Second{:else}Never{/if}";
        let out = render_for_codex_tui(raw);
        assert!(
            out.contains("Unknown condition") && out.contains("First") && out.contains("Second")
        );
        assert!(!out.contains("Never"));
        let locals = vec![];
        assert_eq!(eval("1 === 1.0", &locals, 0), Value::Bool(true));
        assert!(matches!(eval("1 - 2 - 3", &locals, 0), Value::Unknown(_)));
        assert!(matches!(eval("1 == '1'", &locals, 0), Value::Unknown(_)));
    }

    #[test]
    fn nested_layout_does_not_normalize_protected_code() {
        let code = "```text\n    | A | B |\n    |---|---|\n    | 1 | 2 |\n```\n";
        let raw = format!("<box>\n{code}<badge>Ready</badge></box> end");
        let out = render_for_codex_tui(&raw);
        assert!(
            out.contains(code) && out.contains("Ready") && out.ends_with(" end"),
            "{out:?}"
        );
        for (i, _) in raw.char_indices() {
            assert!(out.starts_with(&render_stream_prefix(&raw[..i])), "{i}");
        }
    }
    #[test]
    fn static_values() {
        assert_eq!(render_for_codex_tui("{@body const x = {n: 2}}{x.n}"), "2");
        assert_eq!(render_for_codex_tui("{#if true}yes{:else}no{/if}"), "yes");
        let v = render_for_codex_tui("{#if ready}yes{:else}no{/if}");
        assert!(v.contains("Unknown") && v.contains("yes") && v.contains("no"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_template_placeholders_and_holds_incomplete_markup() {
        let raw = "Hello 한국어 <list connector=\"solid\">{#each items as item}<list-item>{item.label}</list-item><list-item>Visible</list-item>{/each}</list> *{Object.values(checked).length}* {p.n} done";
        let rendered = render_for_codex_tui(raw);
        assert!(rendered.contains("- Visible"));
        assert!(rendered.contains("Hello 한국어"));
        assert!(rendered.contains("[Unavailable: item.label]"));
        assert!(rendered.contains("[Unavailable: p.n]"));
        assert!(rendered.contains("[Unavailable: Object.values(checked).length]"));
        assert!(!rendered.contains("<list"));
        assert!(!rendered.contains("{#each"));
        for (i, _) in raw.char_indices() {
            assert!(
                rendered.starts_with(&render_stream_prefix(&raw[..i])),
                "prefix at {i}"
            );
        }
        assert_eq!(
            render_for_codex_tui("`{item.label}` {name}\n```js\n{p.n}\n```"),
            "`{item.label}` {name}\n```js\n{p.n}\n```"
        );
    }

    #[test]
    fn renders_row_badge_text_link_and_grid_for_codex_tui() {
        let input = r#"## Codex 저장소 확인 결과

<row align="center" gap={2}>
  <badge color="success">정상</badge>
  <text color="secondary" size="sm">2026-11-25 실제 Git 조회 기준</text>
</row>

GitHub의 <Link url="https://github.com/humtr/codex" title="humtr/codex"/> 저장소를 확인했습니다.

<grid columns={2} gap={3}>
  <grid-item>
    <caption>브랜치</caption>
    <text>`main`</text>
  </grid-item>
  <grid-item>
    <caption>HEAD</caption>
    <text>`74bc360b`</text>
  </grid-item>
</grid>"#;

        let rendered = render_for_codex_tui(input);
        assert!(rendered.contains("**[✓ 정상]** · *2026-11-25 실제 Git 조회 기준*"));
        assert!(rendered.contains("[humtr/codex](https://github.com/humtr/codex)"));
        assert!(rendered.contains("| 항목 | 내용 |"));
        assert!(rendered.contains("| 브랜치 | `main` |"));
        assert!(rendered.contains("| HEAD | `74bc360b` |"));
        assert!(!rendered.contains("<row"));
        assert!(!rendered.contains("<badge"));
        assert!(!rendered.contains("<Link"));
        assert!(!rendered.contains("<grid"));
    }

    #[test]
    fn renders_svg_box_divider_and_strips_interactive_dil_directives() {
        let input = r#"{@body const [idx, setIdx] = DIL.useState(0)}
<box title="아키텍처">
<svg viewBox="0 0 340 200">
  <rect x="0" y="0" width="100" height="40"/>
  <text x="50" y="20">AppHost</text>
  <text x="50" y="60">Turn Assembly</text>
</svg>
<divider/>
<slider min={0} max={3} value={idx}/>
<button onClick={() => setIdx(1)}>다음</button>
<checkbox checked={true}>검증 완료</checkbox>
</box>Done<MemoryCite source="x"/>!"#;

        let rendered = render_for_codex_tui(input);
        assert!(rendered.contains("**아키텍처**"));
        assert!(rendered.contains("> **[Diagram]** AppHost · Turn Assembly"));
        assert!(rendered.contains("---"));
        assert!(rendered.contains("- [x] 검증 완료"));
        assert!(rendered.contains("Done!"));
        assert!(!rendered.contains("{@body"));
        assert!(!rendered.contains("<svg"));
        assert!(!rendered.contains("<slider"));
        assert!(!rendered.contains("<button"));
        assert!(!rendered.contains("<MemoryCite"));
    }

    #[test]
    fn preserves_code_blocks_inline_code_generics_and_plain_whitespace_verbatim() {
        let plain = "  Preserved spaces\n다음 줄 OK  ";
        assert_eq!(render_for_codex_tui(plain), plain);

        let code = concat!(
            "Use `<row><badge>x</badge></row>` and `Vec<String>`.\n",
            "```tsx\n",
            "<row align=\"center\">\n",
            "  <badge color=\"success\">literal</badge>\n",
            "</row>\n",
            "```\n",
            "x < 5 and <custom-widget>kept</custom-widget>"
        );
        assert_eq!(render_for_codex_tui(code), code);
    }
}
