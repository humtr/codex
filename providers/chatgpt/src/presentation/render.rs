use super::{Node, Value};

type Attrs = [(String, Value)];

fn get<'a>(attrs: &'a Attrs, key: &str) -> Option<&'a Value> {
    attrs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

fn clean(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_control() || matches!(c, '\n' | '\t'))
        .collect()
}

fn value(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => b.to_string(),
        Value::Number(s) | Value::String(s) => clean(s),
        Value::Unknown(s) => format!("[unknown/unavailable: {}]", clean(s)),
        Value::Array(a) => a.iter().map(value).collect::<Vec<_>>().join(", "),
        Value::Object(o) => o
            .iter()
            .map(|(k, v)| format!("{}: {}", clean(k), value(v)))
            .collect::<Vec<_>>()
            .join(", "),
    }
}

fn attr(a: &Attrs, k: &str) -> String {
    get(a, k).map(value).unwrap_or_default()
}

fn object(v: &Value) -> &Attrs {
    match v {
        Value::Object(o) => o,
        _ => &[],
    }
}

fn array(v: Option<&Value>) -> &[Value] {
    match v {
        Some(Value::Array(a)) => a,
        _ => &[],
    }
}

fn named(n: &Node, tag: &str) -> bool {
    matches!(n, Node::Element { name, .. } if name == tag)
}

fn children(n: &Node) -> &[Node] {
    match n {
        Node::Element { children, .. } => children,
        _ => &[],
    }
}

fn body(ns: &[Node]) -> String {
    ns.iter().map(render_node).collect()
}

fn grouped(ns: &[Node], sep: &str) -> String {
    ns.iter()
        .map(render_node)
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().to_owned())
        .collect::<Vec<_>>()
        .join(sep)
}

fn cell(s: &str) -> String {
    // Preserve existing escaped pipes, including odd/even backslash runs.
    let mut out = String::new();
    let mut slashes = 0;
    for c in s.trim().chars() {
        if c == '|' && slashes % 2 == 0 {
            out.push('\\');
        }
        out.push(if matches!(c, '\n' | '\r') { ' ' } else { c });
        slashes = if c == '\\' { slashes + 1 } else { 0 };
    }
    out
}

fn table(header: &[String], rows: &[Vec<String>]) -> String {
    let row = |r: &[String]| {
        format!(
            "| {} |\n",
            r.iter().map(|s| cell(s)).collect::<Vec<_>>().join(" | ")
        )
    };
    let mut out = format!(
        "\n{}|{}|\n",
        row(header),
        vec!["---"; header.len()].join("|")
    );
    for r in rows {
        out.push_str(&row(r));
    }
    out.push('\n');
    out
}

fn layout_table(ns: &[Node]) -> String {
    // GFM cannot represent spans, mixed header cells or later header rows.
    fn collect(ns: &[Node], rows: &mut Vec<(bool, Vec<String>)>) -> bool {
        for n in ns {
            match n {
                Node::Text(t) if t.trim().is_empty() => {}
                Node::Element {
                    name,
                    attrs,
                    children,
                } if name == "table-section" => {
                    if get(attrs, "header").is_some() || !collect(children, rows) {
                        return false;
                    }
                }
                Node::Element {
                    name,
                    attrs,
                    children,
                } if name == "table-row" => {
                    let header = get(attrs, "header");
                    if get(attrs, "label").is_some()
                        || get(attrs, "labelColSpan").is_some()
                        || header.is_some_and(|v| !matches!(v, Value::Bool(_)))
                        || (!rows.is_empty() && matches!(header, Some(Value::Bool(true))))
                    {
                        return false;
                    }
                    let mut cells = Vec::new();
                    for cell in children {
                        match cell {
                            Node::Text(t) if t.trim().is_empty() => {}
                            Node::Element {
                                name,
                                attrs,
                                children,
                            } if name == "table-cell" => {
                                if ["rowSpan", "colSpan", "header"]
                                    .iter()
                                    .any(|k| get(attrs, k).is_some())
                                {
                                    return false;
                                }
                                cells.push(body(children));
                            }
                            _ => return false,
                        }
                    }
                    if cells.is_empty()
                        || rows
                            .first()
                            .is_some_and(|(_, row)| row.len() != cells.len())
                    {
                        return false;
                    }
                    rows.push((matches!(header, Some(Value::Bool(true))), cells));
                }
                _ => return false,
            }
        }
        true
    }
    fn records(ns: &[Node], out: &mut String, depth: usize) {
        for n in ns {
            if let Node::Element {
                name,
                attrs,
                children,
            } = n
            {
                if matches!(name.as_str(), "table-section" | "table-row" | "table-cell") {
                    let indent = "  ".repeat(depth);
                    let fields = ["header", "label", "labelColSpan", "rowSpan", "colSpan"]
                        .iter()
                        .filter_map(|k| get(attrs, k).map(|v| format!("{k}: {}", value(v))))
                        .collect::<Vec<_>>()
                        .join("; ");
                    let kind = name.trim_start_matches("table-");
                    out.push_str(&format!("{indent}- {kind}"));
                    if !fields.is_empty() {
                        out.push_str(&format!(" [{fields}]"));
                    }
                    if kind == "cell" {
                        out.push_str(&format!(
                            ": {}",
                            body(children)
                                .trim()
                                .replace('\n', &format!("\n{indent}  "))
                        ));
                    }
                    out.push('\n');
                    if kind != "cell" {
                        records(children, out, depth + 1);
                    }
                    continue;
                }
            }
            let content = render_node(n);
            if !content.trim().is_empty() {
                out.push_str(&format!("{}- {}\n", "  ".repeat(depth), content.trim()));
            }
        }
    }
    let mut rows = Vec::new();
    if !collect(ns, &mut rows) || rows.is_empty() {
        let mut out = "\n**[Table: grouped records; spans/layout unavailable]**\n".to_owned();
        records(ns, &mut out, 0);
        out.push('\n');
        return out;
    }
    let width = rows[0].1.len();
    let header = if rows[0].0 {
        rows.remove(0).1
    } else {
        (1..=width).map(|i| format!("Column {i}")).collect()
    };
    table(
        &header,
        &rows.into_iter().map(|(_, r)| r).collect::<Vec<_>>(),
    )
}

fn grid(ns: &[Node]) -> String {
    let items: Vec<_> = ns
        .iter()
        .filter(|n| !matches!(n,Node::Text(t) if t.trim().is_empty()))
        .collect();
    if !items.is_empty()
        && items
            .iter()
            .all(|n| named(n, "grid-item") && children(n).iter().any(|c| named(c, "caption")))
    {
        let rows = items
            .iter()
            .map(|n| {
                let cs = children(n);
                let captions = cs
                    .iter()
                    .filter(|c| named(c, "caption"))
                    .map(|c| body(children(c)))
                    .collect::<Vec<_>>()
                    .join(" · ");
                let rest = cs
                    .iter()
                    .filter(|c| !named(c, "caption"))
                    .map(render_node)
                    .collect::<Vec<_>>()
                    .join(" ");
                vec![captions, rest.trim().into()]
            })
            .collect::<Vec<_>>();
        table(&["항목".into(), "내용".into()], &rows)
    } else {
        grouped(ns, "\n\n")
    }
}

fn chart(a: &Attrs) -> String {
    let Some(content) = get(a, "content") else {
        return "[Chart unavailable: missing content]".into();
    };
    let o = object(content);
    let x = get(o, "xKey")
        .or_else(|| get(o, "nameKey"))
        .and_then(|v| match v {
            Value::String(s) => Some(s.clone()),
            _ => None,
        })
        .unwrap_or_default();
    if get(o, "series").is_some_and(|v| !matches!(v, Value::Array(_))) {
        return format!("[Chart unavailable: malformed series; {}]", value(content));
    }
    let series = array(get(o, "series"));
    let data = array(get(o, "data"));
    if x.is_empty() || data.is_empty() || (series.is_empty() && get(o, "valueKey").is_none()) {
        return format!("[Chart unavailable: {}]", value(content));
    }
    let mut keys = Vec::new();
    let mut header = vec![get(o, "xAxisLabel").map(value).unwrap_or_else(|| x.clone())];
    let mut specs = Vec::new();
    for s in series {
        let p = object(s);
        if !matches!(s, Value::Object(_)) {
            return format!("[Chart unavailable: malformed series; {}]", value(content));
        }
        let key = get(p, "dataKey")
            .or_else(|| get(o, "valueKey"))
            .and_then(|v| match v {
                Value::String(s) => Some(s.clone()),
                _ => None,
            })
            .unwrap_or_default();
        if key.is_empty() {
            return format!("[Chart unavailable: {}]", value(content));
        }
        header.push(get(p, "label").map(value).unwrap_or_else(|| key.clone()));
        keys.push(key);
        specs.push((attr(p, "valuePrefix"), attr(p, "valueSuffix")));
    }
    if keys.is_empty() {
        let Some(Value::String(key)) = get(o, "valueKey") else {
            return format!("[Chart unavailable: {}]", value(content));
        };
        if key.is_empty() {
            return format!("[Chart unavailable: {}]", value(content));
        }
        let key = key.clone();
        header.push(key.clone());
        keys.push(key);
        specs.push((String::new(), String::new()));
    }
    let mut rows = Vec::new();
    for d in data {
        let p = object(d);
        let mut r = vec![get(p, &x).map(value).unwrap_or_else(|| "[missing]".into())];
        for (k, (prefix, suffix)) in keys.iter().zip(&specs) {
            r.push(match get(p, k) {
                Some(Value::Number(n)) => format!("{}{}{}", prefix, clean(n), suffix),
                Some(v) => value(v),
                None => "[missing]".into(),
            });
        }
        // Preserve supplied fields not named by a series, e.g. reference bands.
        rows.push(r);
    }
    let mut out = format!("\n**[Chart: {}]**\n", attr(o, "chartType"));
    out.push_str(&table(&header, &rows));
    for k in [
        "axisLabel",
        "valueSuffix",
        "yAxisMin",
        "yAxisMax",
        "xAxisScale",
        "layout",
    ] {
        if let Some(v) = get(o, k) {
            out.push_str(&format!("{k}: {}\n", value(v)));
        }
    }
    // Expose supplemental data rather than silently discarding it.
    for (i, d) in data.iter().enumerate() {
        let extras = object(d)
            .iter()
            .filter(|(k, _)| k != &x && !keys.contains(k))
            .map(|(k, v)| format!("{}: {}", clean(k), value(v)))
            .collect::<Vec<_>>();
        if !extras.is_empty() {
            out.push_str(&format!("Row {}: {}\n", i + 1, extras.join("; ")));
        }
        if !matches!(d, Value::Object(_)) {
            out.push_str(&format!("Row {}: {}\n", i + 1, value(d)));
        }
    }
    for s in series {
        for k in ["band", "axisLabel", "valueFormat"] {
            if let Some(v) = get(object(s), k) {
                out.push_str(&format!(
                    "{} {k}: {}\n",
                    attr(object(s), "dataKey"),
                    value(v)
                ));
            }
        }
    }
    out
}

fn control(name: &str, a: &Attrs, ns: &[Node]) -> String {
    let mut fields = Vec::new();
    for key in [
        "label",
        "name",
        "placeholder",
        "value",
        "defaultValue",
        "checked",
        "defaultChecked",
        "min",
        "max",
        "step",
        "marks",
        "options",
        "activeIndex",
        "defaultIndex",
        "disabled",
        "required",
    ] {
        if let Some(v) = get(a, key) {
            fields.push(format!("{key}: {}", value(v)));
        }
    }
    let b = grouped(ns, " · ");
    if !b.is_empty() {
        fields.insert(0, b);
    }
    format!(
        "[{name}: static; interaction unavailable{}]",
        if fields.is_empty() {
            String::new()
        } else {
            format!("; {}", fields.join("; "))
        }
    )
}

fn diagram(ns: &[Node]) -> String {
    fn collect(ns: &[Node], labels: &mut Vec<String>, geometry: &mut bool) {
        for n in ns {
            if let Node::Element {
                name, children: cs, ..
            } = n
            {
                if matches!(name.as_str(), "text" | "title" | "desc") {
                    labels.push(body(cs));
                } else {
                    if super::svg_tag(name) {
                        *geometry = true;
                    }
                    collect(cs, labels, geometry);
                }
            } else if let Node::Text(t) = n {
                if !t.trim().is_empty() {
                    labels.push(clean(t));
                }
            }
        }
    }
    let mut labels = Vec::new();
    let mut geometry = false;
    collect(ns, &mut labels, &mut geometry);
    format!(
        "\n> **[Diagram]** {}{}\n",
        labels.join(" · "),
        if geometry || labels.is_empty() {
            " [visual geometry unavailable]"
        } else {
            ""
        }
    )
}

pub(super) fn render_node(node: &Node) -> String {
    let Node::Element {
        name,
        attrs: a,
        children: ns,
    } = node
    else {
        let Node::Text(t) = node else { unreachable!() };
        return clean(t);
    };
    let b = || body(ns);
    let label = || {
        let s = b();
        if s.trim().is_empty() {
            attr(a, "label")
        } else {
            s.trim().to_owned()
        }
    };
    match name.as_str() {
        "MemoryCite" => "[Memory reference unavailable]".into(),
        "spacer" => String::new(),
        "divider" => "\n\n---\n\n".into(),
        "icon" => {
            let s = attr(a, "name");
            if s.is_empty() {
                "[icon unavailable]".into()
            } else {
                format!("[icon: {s}]")
            }
        }
        "title" | "bold" => format!("**{}**", label()),
        "caption" | "italic" => format!("*{}*", label()),
        "text" if attr(a, "color") == "secondary" => {
            let s = label();
            if s.contains('\n') || s.starts_with(['*', '`']) {
                s
            } else {
                format!("*{s}*")
            }
        }
        "text" | "label" | "underline" | "escape" => b(),
        "strikethrough" => format!("~~{}~~", label()),
        "badge" => {
            let p = match attr(a, "color").as_str() {
                "success" => "✓ ",
                "warning" => "⚠ ",
                "danger" | "error" => "✕ ",
                _ => "",
            };
            format!("**[{p}{}]**", label())
        }
        "Link" => {
            let url = get(a, "url").or_else(|| get(a, "href"));
            let mut title = attr(a, "title");
            if title.is_empty() {
                title = label();
            }
            if let Some(Value::String(u)) = url {
                if (u.starts_with("https://") || u.starts_with("http://"))
                    && u.split_once("://")
                        .is_some_and(|(_, r)| !r.is_empty() && !r.starts_with('/'))
                    && !title.chars().any(|c| c.is_control())
                    && !u.chars().any(|c| {
                        c.is_whitespace()
                            || c.is_control()
                            || matches!(c, '(' | ')' | '<' | '>' | '\\')
                    })
                {
                    if title.is_empty() {
                        title = u.clone();
                    }
                    return format!(
                        "[{}]({u})",
                        title
                            .replace('\\', "\\\\")
                            .replace('[', "\\[")
                            .replace(']', "\\]")
                    );
                }
            }
            format!(
                "{title} [link unavailable{}]",
                url.map(|v| format!(": {}", value(v).replace('[', "\\[").replace(']', "\\]")))
                    .unwrap_or_default()
            )
        }
        "row" => grouped(ns, " · "),
        "grid" => grid(ns),
        "grid-item" | "col" | "flow" | "flow-item" | "card" | "box" | "carousel-item" => {
            let title = attr(a, "title");
            let s = b();
            format!(
                "\n{}{}\n",
                if title.is_empty() {
                    String::new()
                } else {
                    format!("**{title}**\n")
                },
                s.trim_matches('\n')
            )
        }
        "list" => grouped(ns, "\n"),
        "radio-group" | "carousel" => control(name, a, ns),
        "list-item" => format!("- {}\n", label().replace('\n', "\n  ")),
        "checkbox" => {
            let v = get(a, "checked").or_else(|| get(a, "defaultChecked"));
            let mark = match v {
                Some(Value::Bool(true)) => "x",
                Some(Value::Bool(false)) | None => " ",
                _ => "?",
            };
            format!(
                "- [{mark}] {} [static; interaction unavailable{}]\n",
                label(),
                if mark == "?" {
                    format!("; {}", v.map(value).unwrap_or_default())
                } else {
                    String::new()
                }
            )
        }
        "table" => layout_table(ns),
        "table-section" | "table-row" | "table-cell" => grouped(ns, " · "),
        "Chart" => chart(a),
        "svg" => diagram(ns),
        "math" => {
            let s = get(a, "value").map(value).unwrap_or_else(b);
            if matches!(get(a, "value"), Some(Value::Unknown(_))) {
                s
            } else if matches!(get(a, "block"), Some(Value::Bool(true))) {
                format!("\n$$\n{s}\n$$\n")
            } else {
                format!("${s}$")
            }
        }
        "button" | "slider" | "segmented-control" | "select" | "input" | "textarea" | "radio"
        | "date-picker" | "form" | "pressable" | "popover" | "popover-trigger"
        | "popover-content" => control(name, a, ns),
        _ => {
            let fields = a
                .iter()
                .filter(|(k, _)| {
                    !matches!(
                        k.as_str(),
                        "onClick" | "onChange" | "onSubmit" | "onHover" | "id"
                    )
                })
                .map(|(k, v)| format!("{}: {}", clean(k), value(v)))
                .collect::<Vec<_>>()
                .join("; ");
            format!(
                "[{} unavailable{}]{}",
                clean(name),
                if fields.is_empty() {
                    String::new()
                } else {
                    format!(": {fields}")
                },
                b()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn t(s: &str) -> Node {
        Node::Text(s.into())
    }
    fn s(s: &str) -> Value {
        Value::String(s.into())
    }
    fn e(name: &str, attrs: Vec<(&str, Value)>, children: Vec<Node>) -> Node {
        Node::Element {
            name: name.into(),
            attrs: attrs.into_iter().map(|(k, v)| (k.into(), v)).collect(),
            children,
        }
    }
    fn obj(fields: Vec<(&str, Value)>) -> Value {
        Value::Object(fields.into_iter().map(|(k, v)| (k.into(), v)).collect())
    }
    #[test]
    fn selections_and_complex_tables() {
        let project = super::super::render_for_codex_tui;
        for (input, expected) in [
            (
                r#"<radio-group value="b"><radio value="a">A</radio><radio value="b">B</radio></radio-group>"#,
                vec!["radio-group: static", "value: b", "value: a", "A", "B"],
            ),
            (
                r#"<carousel activeIndex={1}><carousel-item>First</carousel-item><carousel-item>Second</carousel-item></carousel>"#,
                vec!["carousel: static", "activeIndex: 1", "First", "Second"],
            ),
            (
                r#"<table><table-section header><table-row><table-cell colSpan={2}>Heading</table-cell></table-row><table-row><table-cell>Subhead</table-cell></table-row></table-section><table-row label="Record" labelColSpan={2}><table-cell rowSpan={2}>Value</table-cell></table-row></table>"#,
                vec![
                    "grouped records",
                    "header: true",
                    "colSpan: 2",
                    "Heading",
                    "Subhead",
                    "label: Record",
                    "labelColSpan: 2",
                    "rowSpan: 2",
                    "Value",
                ],
            ),
            ("<MemoryCite />", vec!["Memory reference unavailable"]),
        ] {
            let out = project(input);
            for part in expected {
                assert!(out.contains(part), "missing {part}: {out}");
            }
        }
    }

    #[test]
    fn malformed_chart_metadata_is_disclosed() {
        let project = super::super::render_for_codex_tui;
        let out = project(
            r#"<Chart content={{"nameKey":"name","valueKey":"count","series":"broken","data":[{"name":"A","count":42}]}}/>"#,
        );
        assert!(
            out.contains("unavailable") && out.contains("broken") && out.contains("42"),
            "{out}"
        );
    }

    #[test]
    fn row_grid_and_badge() {
        let row = e(
            "row",
            vec![],
            vec![
                e("badge", vec![("color", s("success"))], vec![t("정상")]),
                e("text", vec![("color", s("secondary"))], vec![t("Now")]),
            ],
        );
        assert_eq!(render_node(&row), "**[✓ 정상]** · *Now*");
        let grid = e(
            "grid",
            vec![],
            vec![e(
                "grid-item",
                vec![],
                vec![
                    e("caption", vec![], vec![t("HEAD")]),
                    e("text", vec![], vec![t("`main`")]),
                ],
            )],
        );
        assert!(render_node(&grid).contains("| HEAD | `main` |"));
    }
    #[test]
    fn charts_keep_sign_zero_null_and_missing() {
        let data = vec![
            obj(vec![
                ("name", s("A")),
                ("count", Value::Number("-2".into())),
            ]),
            obj(vec![("name", s("B")), ("count", Value::Number("0".into()))]),
            obj(vec![("name", s("C")), ("count", Value::Null)]),
            obj(vec![("name", s("D"))]),
        ];
        let content = obj(vec![
            ("chartType", s("bar")),
            ("xKey", s("name")),
            (
                "series",
                Value::Array(vec![obj(vec![
                    ("dataKey", s("count")),
                    ("label", s("Count")),
                    ("valueSuffix", s(" kg")),
                ])]),
            ),
            ("data", Value::Array(data)),
        ]);
        let out = render_node(&e("Chart", vec![("content", content)], vec![]));
        for expected in [
            "Count",
            "| A | -2 kg |",
            "| B | 0 kg |",
            "| C | null |",
            "| D | [missing] |",
        ] {
            assert!(out.contains(expected), "{out}");
        }
    }
    #[test]
    fn table_and_layout() {
        let row = e(
            "table-row",
            vec![],
            vec![
                e("table-cell", vec![], vec![t("A|B")]),
                e("table-cell", vec![], vec![t("6")]),
            ],
        );
        assert!(render_node(&e("table", vec![], vec![row])).contains("| A\\|B | 6 |"));
        let out = super::super::render_for_codex_tui(
            "<box>\n    | Item | Count |\n    |---|---:|\n    | Done | 6 |\n\n    code\n</box>",
        );
        assert!(out.contains("\n| Item | Count |\n|---|---:|\n| Done | 6 |"));
        assert!(out.contains("\n    code\n"));
    }
    #[test]
    fn static_and_safe_links() {
        let out = render_node(&e(
            "button",
            vec![("onClick", Value::Unknown("callback".into()))],
            vec![t("Details")],
        ));
        assert!(out.contains("Details") && out.contains("static") && !out.contains("callback"));
        let out = render_node(&e(
            "Link",
            vec![
                ("url", Value::Unknown("item.url".into())),
                ("title", s("Details")),
            ],
            vec![],
        ));
        assert!(out.contains("unavailable") && !out.contains("](item.url)"));
        let out = render_node(&e("Link", vec![("url", s("javascript:alert(1)"))], vec![]));
        assert!(!out.contains("]("));
        assert_eq!(render_node(&t("a\x1b[31m\x07b")), "a[31mb");
    }
    #[test]
    fn diagram_and_math() {
        let d = e(
            "svg",
            vec![],
            vec![
                e("text", vec![], vec![t("AppHost")]),
                e("text", vec![], vec![t("Turn Assembly")]),
            ],
        );
        assert!(render_node(&d).contains("> **[Diagram]** AppHost · Turn Assembly"));
        assert_eq!(
            render_node(&e("math", vec![("value", s("x^2"))], vec![])),
            "$x^2$"
        );
        assert!(render_node(&e(
            "slider",
            vec![
                ("min", Value::Number("0".into())),
                ("value", Value::Unknown("idx".into())),
                ("max", Value::Number("3".into()))
            ],
            vec![]
        ))
        .contains("unknown"));
        assert!(render_node(&e(
            "checkbox",
            vec![("checked", Value::Bool(true))],
            vec![t("Done")]
        ))
        .contains("- [x] Done"));
    }
}
