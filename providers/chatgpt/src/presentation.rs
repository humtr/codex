//! Narrow ChatGPT DIL (Declarative Interface Language) to Codex-native GFM Markdown projection.
//!
//! Converts only allowlisted ChatGPT UI layout/component tags outside code blocks and inline
//! code spans into clean CommonMark/GFM structures that official Codex TUI renders natively
//! (links, bold status badges, tables, horizontal rules, blockquotes). Never mutates fenced
//! code blocks, inline code spans, unknown HTML/XML, Rust generics, or tool arguments.

/// Render allowlisted ChatGPT DIL components into Codex-native GFM Markdown.
pub fn render_for_codex_tui(input: &str) -> String {
    if !input.contains('<') && !input.contains('{') {
        return input.to_owned();
    }

    let mut out = String::with_capacity(input.len());
    let mut fence: Option<(char, usize)> = None;
    let mut prose_buf = String::new();
    let mut saw_dil = false;

    for line in input.split_inclusive('\n') {
        let trimmed = line.trim_start_matches(' ');
        let indent = line.len() - trimmed.len();
        let first = trimmed.chars().next().unwrap_or('\0');
        let run = trimmed.chars().take_while(|&c| c == first).count();

        if indent <= 3 && matches!(first, '`' | '~') && run >= 3 {
            match fence {
                None => {
                    if !prose_buf.is_empty() {
                        let (rendered, changed) = render_prose_segment(&prose_buf);
                        saw_dil |= changed;
                        out.push_str(&rendered);
                        prose_buf.clear();
                    }
                    fence = Some((first, run));
                    out.push_str(line);
                    continue;
                }
                Some((ch, n)) if ch == first && run >= n && trimmed[run..].trim().is_empty() => {
                    fence = None;
                    out.push_str(line);
                    continue;
                }
                _ => {}
            }
        }

        if fence.is_some() {
            out.push_str(line);
        } else {
            prose_buf.push_str(line);
        }
    }

    if !prose_buf.is_empty() {
        let (rendered, changed) = render_prose_segment(&prose_buf);
        saw_dil |= changed;
        out.push_str(&rendered);
    }

    if saw_dil {
        out
    } else {
        input.to_owned()
    }
}

fn render_prose_segment(input: &str) -> (String, bool) {
    let mut out = String::with_capacity(input.len());
    let mut changed = false;
    let bytes = input.as_bytes();
    let mut i = 0usize;
    let mut inline_code = 0usize;

    while i < bytes.len() {
        let rest = &input[i..];

        // Track inline code spans verbatim.
        if bytes[i] == b'`' {
            let n = rest.bytes().take_while(|&b| b == b'`').count();
            if inline_code == 0 {
                inline_code = n;
            } else if inline_code == n {
                inline_code = 0;
            }
            out.push_str(&rest[..n]);
            i += n;
            continue;
        }

        if inline_code > 0 {
            let ch = rest.chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
            continue;
        }

        // Drop unevaluated template expressions, never execute them. Remove
        // surrounding emphasis when the expression is its entire content.
        let emphasis = if rest.starts_with("**{") {
            2
        } else if rest.starts_with("*{") {
            1
        } else {
            0
        };
        if let Some(end) = template_end(&rest[emphasis..]) {
            let end = end + emphasis;
            if emphasis == 0 || rest[end..].starts_with(&rest[..emphasis]) {
                changed = true;
                i += end + emphasis;
                continue;
            }
        }

        if bytes[i] == b'<' {
            if let Some((consumed, replacement)) = try_render_dil_element(rest) {
                changed = true;
                out.push_str(&replacement);
                i += consumed;
                continue;
            }
        }

        let ch = rest.chars().next().unwrap();
        out.push(ch);
        i += ch.len_utf8();
    }

    (out, changed)
}

// Recognize directive syntax or a JS member/call expression, not ordinary
// braces such as `{name}`, LaTeX groups, JSON objects or Rust blocks.
fn template_end(rest: &str) -> Option<usize> {
    let body = rest.strip_prefix('{')?;
    let mut depth = 1;
    let mut quote = None;
    let mut escaped = false;
    for (i, ch) in body.char_indices() {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    let expr = body[..i].trim();
                    let member = expr.split_once('.').is_some_and(|(root, tail)| {
                        !root.is_empty()
                            && root.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
                            && tail.starts_with(|c: char| c.is_ascii_alphabetic() || c == '_')
                            && (!expr.contains(char::is_whitespace) || expr.contains('('))
                    });
                    return (expr.starts_with(['@', '#', ':', '/']) || member).then_some(i + 2);
                }
            }
            _ => {}
        }
    }
    None
}

/// A conservative stable projection: do not publish an unfinished template,
/// component or code construct. The same renderer handles the settled text.
pub fn render_stream_prefix(input: &str) -> String {
    let mut i = 0;
    while i < input.len() {
        let rest = &input[i..];
        let ch = rest.chars().next().unwrap();
        match ch {
            '<' => {
                if let Some((n, _)) = try_render_dil_element(rest) {
                    i += n;
                    continue;
                }
                // Unknown complete HTML/generics pass through at settlement;
                // a partial tag may still become an allowlisted component.
                break;
            }
            '{' => {
                if let Some(n) = template_end(rest) {
                    i += n;
                    continue;
                }
                break;
            }
            '*' => {
                // An emphasis marker might introduce an unevaluated expression.
                let n = rest.bytes().take_while(|&b| b == b'*').count();
                if n == rest.len() {
                    break;
                }
                if rest[n..].starts_with('{') {
                    let Some(end) = template_end(&rest[n..]) else {
                        break;
                    };
                    if !rest[n + end..].starts_with(&rest[..n]) {
                        break;
                    }
                    i += n + end + n;
                    continue;
                }
            }
            // Buffer code constructs as a whole rather than misread literal
            // DIL in a fence or inline span. Ordinary prose streams immediately.
            '`' | '~' => break,
            _ => {}
        }
        i += ch.len_utf8();
    }
    render_for_codex_tui(&input[..i])
}

const DIL_BLOCK_TAGS: &[&str] = &[
    "svg",
    "grid",
    "grid-item",
    "row",
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
];

const DIL_VOID_OR_SELF_TAGS: &[&str] =
    &["Link", "MemoryCite", "divider", "spacer", "icon", "slider"];

fn parse_open_tag(rest: &str) -> Option<(&str, &str, usize, bool)> {
    if !rest.starts_with('<') || rest.starts_with("</") || rest.starts_with("<!") {
        return None;
    }
    let after_lt = &rest[1..];
    let name_len = after_lt
        .bytes()
        .take_while(|b| b.is_ascii_alphanumeric() || *b == b'-' || *b == b'_')
        .count();
    if name_len == 0 {
        return None;
    }
    let name = &after_lt[..name_len];
    let next_byte = after_lt.as_bytes().get(name_len).copied();
    if !matches!(next_byte, Some(b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>')) {
        return None;
    }
    if !DIL_BLOCK_TAGS.contains(&name) && !DIL_VOID_OR_SELF_TAGS.contains(&name) {
        return None;
    }
    let end = find_tag_end(rest)?;
    let raw_tag = &rest[..end];
    let self_closing = raw_tag[..end - 1].trim_end().ends_with('/');
    let attrs_end = if self_closing {
        raw_tag[..end - 1].trim_end().len().saturating_sub(1)
    } else {
        end - 1
    };
    let attrs = raw_tag.get(1 + name_len..attrs_end).unwrap_or("").trim();
    Some((name, attrs, end, self_closing))
}

/// Find the closing `>` of a JSX/DIL opening tag, respecting quotes and `{...}` expressions.
fn find_tag_end(rest: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut brace_depth = 0usize;
    let mut escaped = false;
    for (idx, ch) in rest.char_indices().take_while(|(idx, _)| *idx < 8192) {
        if let Some(q) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == q {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' => quote = Some(ch),
            '{' => brace_depth += 1,
            '}' => brace_depth = brace_depth.saturating_sub(1),
            '>' if brace_depth == 0 => return Some(idx + 1),
            _ => {}
        }
    }
    None
}

/// Find the matching `</name>` closing tag respecting nested `<name ...>` blocks.
fn find_matching_close(rest: &str, name: &str, content_start: usize) -> Option<(usize, usize)> {
    let mut depth = 1usize;
    let mut i = content_start;
    let bytes = rest.as_bytes();
    let open_prefix = format!("<{name}");
    let close_prefix = format!("</{name}");

    while i < bytes.len() {
        if bytes[i] == b'<' {
            let slice = &rest[i..];
            if slice.starts_with(&close_prefix)
                && matches!(
                    slice.as_bytes().get(close_prefix.len()),
                    Some(b'>' | b' ' | b'\t' | b'\n' | b'\r')
                )
            {
                if let Some(end) = slice.find('>') {
                    depth -= 1;
                    if depth == 0 {
                        return Some((i, i + end + 1));
                    }
                    i += end + 1;
                    continue;
                }
            } else if slice.starts_with(&open_prefix)
                && matches!(
                    slice.as_bytes().get(open_prefix.len()),
                    Some(b' ' | b'\t' | b'\n' | b'\r' | b'/' | b'>')
                )
            {
                if let Some(end) = find_tag_end(slice) {
                    let self_closing = slice[..end - 1].trim_end().ends_with('/');
                    if !self_closing {
                        depth += 1;
                    }
                    i += end;
                    continue;
                }
            }
        }
        let ch = rest[i..].chars().next().unwrap();
        i += ch.len_utf8();
    }
    None
}

fn attr_value<'a>(attrs: &'a str, key: &str) -> Option<&'a str> {
    let bytes = attrs.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        let key_start = i;
        while i < bytes.len()
            && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_')
        {
            i += 1;
        }
        let found_key = &attrs[key_start..i];
        if found_key.is_empty() {
            if i < bytes.len() {
                i += attrs[i..].chars().next().map_or(1, char::len_utf8);
            }
            continue;
        }
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() || bytes[i] != b'=' {
            continue;
        }
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_whitespace() {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let val = match bytes[i] {
            b'"' | b'\'' => {
                let q = bytes[i];
                i += 1;
                let v_start = i;
                while i < bytes.len() && bytes[i] != q {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
                let v = &attrs[v_start..i];
                if i < bytes.len() {
                    i += 1;
                }
                Some(v)
            }
            b'{' => {
                i += 1;
                let v_start = i;
                let mut depth = 1usize;
                while i < bytes.len() && depth > 0 {
                    match bytes[i] {
                        b'{' => depth += 1,
                        b'}' => depth -= 1,
                        _ => {}
                    }
                    i += 1;
                }
                Some(attrs[v_start..i.saturating_sub(1)].trim())
            }
            _ => {
                let v_start = i;
                while i < bytes.len() && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                Some(&attrs[v_start..i])
            }
        };
        if found_key == key {
            return val;
        }
    }
    None
}

fn try_render_dil_element(rest: &str) -> Option<(usize, String)> {
    let (name, attrs, open_end, self_closing) = parse_open_tag(rest)?;

    if self_closing {
        let rendered = match name {
            "MemoryCite" | "spacer" | "icon" | "slider" => String::new(),
            "divider" => "\n\n---\n\n".to_owned(),
            "Link" => render_link(attrs, ""),
            _ => String::new(),
        };
        return Some((open_end, rendered));
    }

    // Handle stray/unpaired void tags defensively if written without `/`.
    if matches!(name, "divider" | "spacer" | "icon" | "slider") {
        let rendered = if name == "divider" {
            "\n\n---\n\n".to_owned()
        } else {
            String::new()
        };
        return Some((open_end, rendered));
    }

    let (inner_end, total_end) = find_matching_close(rest, name, open_end)?;
    let inner_raw = &rest[open_end..inner_end];

    let rendered = match name {
        "MemoryCite" => String::new(),
        "Link" => render_link(attrs, inner_raw.trim()),
        "badge" => render_badge(attrs, inner_raw),
        "title" => {
            let (inner, _) = render_prose_segment(inner_raw);
            let t = inner.trim();
            if t.is_empty() {
                String::new()
            } else {
                format!("**{t}**")
            }
        }
        "caption" => {
            let (inner, _) = render_prose_segment(inner_raw);
            let t = inner.trim();
            if t.is_empty() {
                String::new()
            } else {
                format!("*{t}*")
            }
        }
        "text" => render_text(attrs, inner_raw),
        "row" => render_row(inner_raw),
        "grid" => render_grid(inner_raw),
        "grid-item" => {
            let (inner, _) = render_prose_segment(inner_raw);
            inner.trim().to_owned()
        }
        "list" => render_prose_segment(inner_raw).0,
        "list-item" => {
            let inner = render_prose_segment(inner_raw).0;
            if inner.trim().is_empty() {
                String::new()
            } else {
                format!("- {}\n", inner.trim())
            }
        }
        "box" | "card" => render_container(attrs, inner_raw),
        "svg" => render_svg(inner_raw),
        "checkbox" => {
            let (inner, _) = render_prose_segment(inner_raw);
            let label = inner.trim();
            if label.is_empty() {
                String::new()
            } else {
                let checked = attr_value(attrs, "checked") == Some("true");
                let mark = if checked { "x" } else { " " };
                format!("- [{mark}] {label}\n")
            }
        }
        "button" => String::new(),
        _ => return None,
    };

    Some((total_end, rendered))
}

fn render_link(attrs: &str, fallback_label: &str) -> String {
    let url = attr_value(attrs, "url")
        .or_else(|| attr_value(attrs, "href"))
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let title = attr_value(attrs, "title")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| (!fallback_label.is_empty()).then_some(fallback_label));
    match (title, url) {
        (Some(t), Some(u)) => format!("[{t}]({u})"),
        (None, Some(u)) => format!("[{u}]({u})"),
        (Some(t), None) => t.to_owned(),
        (None, None) => String::new(),
    }
}

fn render_badge(attrs: &str, inner_raw: &str) -> String {
    let (inner, _) = render_prose_segment(inner_raw);
    let label = inner.trim();
    if label.is_empty() {
        return String::new();
    }
    let prefix = match attr_value(attrs, "color").map(str::trim) {
        Some("success") => "✓ ",
        Some("warning") => "⚠ ",
        Some("danger" | "error") => "✕ ",
        _ => "",
    };
    format!("**[{prefix}{label}]**")
}

fn render_text(attrs: &str, inner_raw: &str) -> String {
    let (inner, _) = render_prose_segment(inner_raw);
    let trimmed = inner.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    if attr_value(attrs, "color") == Some("secondary")
        && !trimmed.contains('\n')
        && !trimmed.starts_with('*')
        && !trimmed.starts_with('`')
    {
        format!("*{trimmed}*")
    } else {
        trimmed.to_owned()
    }
}

fn render_row(inner_raw: &str) -> String {
    let (rendered, _) = render_prose_segment(inner_raw);
    let pieces: Vec<&str> = rendered
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    if pieces.is_empty() {
        String::new()
    } else {
        pieces.join(" · ")
    }
}

fn render_grid(inner_raw: &str) -> String {
    // Extract direct <grid-item> children into a clean GFM table when structured as caption+value.
    let mut items: Vec<(Option<String>, String)> = Vec::new();
    let mut i = 0usize;
    while i < inner_raw.len() {
        if let Some(pos) = inner_raw[i..].find("<grid-item") {
            let start = i + pos;
            let slice = &inner_raw[start..];
            if let Some((_, _, open_end, false)) = parse_open_tag(slice) {
                if let Some((inner_end, total_end)) =
                    find_matching_close(slice, "grid-item", open_end)
                {
                    let body = &slice[open_end..inner_end];
                    items.push(extract_grid_item(body));
                    i = start + total_end;
                    continue;
                }
            }
            i = start + 1;
        } else {
            break;
        }
    }

    if items.is_empty() {
        let (rendered, _) = render_prose_segment(inner_raw);
        return rendered.trim().to_owned();
    }

    let all_have_captions = items.iter().all(|(cap, _)| cap.is_some());
    if all_have_captions {
        let mut out = String::from("\n| 항목 | 내용 |\n|---|---|\n");
        for (cap, val) in items {
            let k = escape_table_cell(cap.as_deref().unwrap_or(""));
            let v = escape_table_cell(&val);
            out.push_str(&format!("| {k} | {v} |\n"));
        }
        out
    } else {
        let mut out = String::from("\n");
        for (cap, val) in items {
            match (cap, val.is_empty()) {
                (Some(k), false) => out.push_str(&format!("- **{k}:** {val}\n")),
                (Some(k), true) => out.push_str(&format!("- **{k}**\n")),
                (None, false) => out.push_str(&format!("- {val}\n")),
                (None, true) => {}
            }
        }
        out
    }
}

fn extract_grid_item(body: &str) -> (Option<String>, String) {
    let mut caption: Option<String> = None;
    let mut rest_buf = String::with_capacity(body.len());
    let mut i = 0usize;
    while i < body.len() {
        if body.as_bytes()[i] == b'<' && body[i..].starts_with("<caption") && caption.is_none() {
            if let Some((_, _, open_end, false)) = parse_open_tag(&body[i..]) {
                if let Some((inner_end, total_end)) =
                    find_matching_close(&body[i..], "caption", open_end)
                {
                    let (cap_rendered, _) =
                        render_prose_segment(&body[i + open_end..i + inner_end]);
                    let cap = cap_rendered.trim().to_owned();
                    if !cap.is_empty() {
                        caption = Some(cap);
                    }
                    i += total_end;
                    continue;
                }
            }
        }
        let ch = body[i..].chars().next().unwrap();
        rest_buf.push(ch);
        i += ch.len_utf8();
    }
    let (val_rendered, _) = render_prose_segment(&rest_buf);
    let val = val_rendered
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(" ");
    (caption, val)
}

fn escape_table_cell(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

fn render_container(attrs: &str, inner_raw: &str) -> String {
    let (inner, _) = render_prose_segment(inner_raw);
    let trimmed = inner.trim();
    let title = attr_value(attrs, "title")
        .map(str::trim)
        .filter(|s| !s.is_empty());
    let mut out = String::new();
    if let Some(t) = title {
        out.push_str("\n**");
        out.push_str(t);
        out.push_str("**\n");
    }
    if !trimmed.is_empty() {
        out.push('\n');
        out.push_str(trimmed);
        out.push('\n');
    }
    out
}

fn render_svg(inner_raw: &str) -> String {
    let mut labels: Vec<String> = Vec::new();
    let mut i = 0usize;
    while i < inner_raw.len() {
        if let Some(pos) = inner_raw[i..].find("<text") {
            let start = i + pos;
            let slice = &inner_raw[start..];
            if let Some(open_end) = find_tag_end(slice) {
                if let Some((inner_end, total_end)) = find_matching_close(slice, "text", open_end) {
                    let label = render_prose_segment(&slice[open_end..inner_end]).0;
                    if !label.trim().is_empty() {
                        labels.push(label.trim().to_owned());
                    }
                    i = start + total_end;
                    continue;
                }
            }
            i = start + 1;
        } else {
            break;
        }
    }
    if labels.is_empty() {
        String::new()
    } else {
        format!("\n> **[Diagram]** {}\n", labels.join(" · "))
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
        assert!(!rendered.contains("{item.label}"));
        assert!(!rendered.contains("{p.n}"));
        assert!(!rendered.contains("Object.values"));
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
