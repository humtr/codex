//! Additional independent semantic/prefix oracle. Run as a standalone binary
//! until the integrated Phase 1 implementation qualifies; no live account.
#[path = "../../src/presentation.rs"]
mod presentation;

use presentation::{render_for_codex_tui, render_stream_prefix};
use std::time::Instant;

fn prefix_check(input: &str, settled: &str) -> Option<usize> {
    let mut previous = String::new();
    for i in input
        .char_indices()
        .map(|(i, _)| i)
        .chain(std::iter::once(input.len()))
    {
        let live = render_stream_prefix(&input[..i]);
        if !live.starts_with(&previous) || !settled.starts_with(&live) {
            return Some(i);
        }
        previous = live;
    }
    None
}

fn check(name: &str, input: &str, complete_live: bool, expect: impl FnOnce(&str) -> bool) -> bool {
    let settled = render_for_codex_tui(input);
    let live = render_stream_prefix(input);
    let unstable = prefix_check(input, &settled);
    let semantics = expect(&settled);
    let passed = semantics && unstable.is_none() && (!complete_live || live == settled);
    println!(
        "{} {name}: semantics={semantics}, unstable_at={unstable:?}, live_complete={}",
        if passed { "PASS" } else { "FAIL" },
        live == settled
    );
    if !passed {
        println!("  settled={settled:?}\n  live={live:?}");
    }
    passed
}

fn contains_all(text: &str, expected: &[&str]) -> bool {
    expected.iter().all(|part| text.contains(part))
}

#[cfg_attr(test, test)]
fn main() {
    let mut results = Vec::new();

    // Ordinary source text stays exact, even when it contains recognized names.
    for (name, raw) in [
        (
            "escaped_literal",
            r"Literal: \<badge>literal</badge> after.",
        ),
        (
            "unknown_xml",
            "<example><badge>literal</badge></example> after.",
        ),
        (
            "html_comment",
            "<!-- <badge>literal</badge> -->\nAfter comment.",
        ),
        (
            "double_backtick",
            "Use ``<badge>`literal`</badge>`` then continue.",
        ),
        (
            "list_nested_fence",
            "- Example:\n  ```xml\n  <badge>literal</badge>\n  ```\n\nAfter list.",
        ),
        (
            "literal_indented_gfm_ko",
            "    | 작업 | 완료 건수 |\n    |---|---:|\n    | 구현 | 6 |",
        ),
        (
            "ordinary_gfm_ko",
            "| 작업 | 완료 건수 |\n|---|---:|\n| 구현 | 6 |",
        ),
        (
            "math_literal",
            "Math: $x^{2} + y^{2}$ and \\(a < b\\). End.",
        ),
        (
            "mermaid_fence",
            "```mermaid\nflowchart TD\n A[Input] --> B[Output]\n```\nAfter graph.",
        ),
    ] {
        results.push(check(name, raw, true, |out| out == raw));
    }

    let selector = r#"<segmented-control options={[{"label":"요약 보기","value":"summary"},{"label":"상세 보기","value":"detail"}]} value={view} onChange={setView}/>"#;
    results.push(check("historical_selector_ko", selector, false, |out| {
        contains_all(out, &["요약 보기", "상세 보기"])
            && out.to_ascii_lowercase().contains("unknown")
            && !out.contains("<segmented-control")
    }));
    let table = "<table><table-row><table-cell>구현 | 검증</table-cell><table-cell>6</table-cell></table-row></table>";
    results.push(check("gfm_pipe_escape", table, false, |out| {
        contains_all(out, &["구현 \\| 검증", "6", "|"]) && !out.contains("<table")
    }));
    let chart = r#"<Chart content={{"chartType":"bar","xKey":"name","series":[{"dataKey":"t","label":"Temperature","valueSuffix":" °C"}],"data":[{"name":"A","t":-2},{"name":"B","t":0},{"name":"C","t":null}]}}/>"#;
    results.push(check("chart_labels_units_sign_null", chart, false, |out| {
        let lower = out.to_ascii_lowercase();
        contains_all(
            out,
            &["Temperature", "°C", "| A |", "-2", "| B |", "0", "| C |"],
        ) && ["null", "missing", "unavailable"]
            .iter()
            .any(|s| lower.contains(s))
            && !out.contains("<Chart")
    }));
    let state = "{@body const [idx, setIdx] = DIL.useState(0)}\n{#if idx === 0}Selected initial{:else}Wrong branch{/if}";
    results.push(check("known_initial_state", state, false, |out| {
        out.contains("Selected initial") && !out.contains("Wrong branch")
    }));
    let control = "<slider min={0} max={3} value={1}/>";
    results.push(check(
        "static_slider_bounds_default",
        control,
        false,
        |out| contains_all(out, &["0", "3", "1"]) && out.to_ascii_lowercase().contains("static"),
    ));
    let unsafe_link = r#"<Link url="javascript:alert(1)" title="Danger"/>"#;
    results.push(check("unsafe_link", unsafe_link, false, |out| {
        let lower = out.to_ascii_lowercase();
        out.contains("Danger")
            && !lower.contains("](javascript:")
            && ["unavailable", "unsafe", "unsupported"]
                .iter()
                .any(|s| lower.contains(s))
    }));
    let terminal = r#"<Link url="https://example.org/" title={"Safe\u001b[31mTitle"}/>"#;
    results.push(check("attribute_terminal_escape", terminal, false, |out| {
        contains_all(out, &["Safe", "Title"]) && !out.contains('\u{1b}') && !out.contains('\u{7}')
    }));
    results.push(check(
        "malformed_element",
        "Prefix <badge>Unclosed 끝",
        false,
        |out| contains_all(out, &["Prefix", "Unclosed 끝"]),
    ));
    results.push(check(
        "malformed_chart",
        r#"Prefix <Chart content={{"data":[{"name":"recoverable-label","value":42}"#,
        false,
        |out| contains_all(out, &["Prefix", "recoverable-label", "42"]),
    ));

    // Deterministic bounded prefix corpus. Inputs are synthetic and only the
    // prefix invariant, not an invented semantic interpretation, is asserted.
    let tokens = [
        "a",
        "한",
        "😀",
        " ",
        "\n",
        "<",
        ">",
        "{",
        "}",
        "`",
        "~",
        "*",
        "<badge>",
        "</badge>",
        "{#if true}",
        "{/if}",
    ];
    let mut seed = 0x69e7_52c4_u32;
    let mut prefix_failures = 0;
    for case in 0..48 {
        let mut raw = String::new();
        for _ in 0..18 {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            raw.push_str(tokens[(seed as usize) % tokens.len()]);
        }
        let settled = render_for_codex_tui(&raw);
        if let Some(i) = prefix_check(&raw, &settled) {
            prefix_failures += 1;
            println!("FAIL deterministic_prefix_{case} at {i}: {raw:?}");
        }
    }
    println!("deterministic_prefix: {}/48 stable", 48 - prefix_failures);
    results.push(prefix_failures == 0);

    // Measure deep/large cases before choosing limits. Avoid quadratic testing
    // every byte of large input; verify representative checkpoints instead.
    for (name, raw, sentinel) in [
        (
            "deep_128",
            format!("{}DEEP_VALUE{}", "<box>".repeat(128), "</box>".repeat(128)),
            "DEEP_VALUE",
        ),
        (
            "deep_512",
            format!("{}DEEP_VALUE{}", "<box>".repeat(512), "</box>".repeat(512)),
            "DEEP_VALUE",
        ),
        (
            "large_plain",
            format!("{}LARGE_VALUE", "한😀 A < 5 and {name}.\n".repeat(4096)),
            "LARGE_VALUE",
        ),
        (
            "large_repeated",
            format!("{}LARGE_VALUE", "<badge>한😀</badge> ".repeat(4096)),
            "LARGE_VALUE",
        ),
    ] {
        let t = Instant::now();
        let settled = render_for_codex_tui(&raw);
        let live = render_stream_prefix(&raw);
        let passed = settled.contains(sentinel) && settled.starts_with(&live);
        println!(
            "{} {name}: input_bytes={} settled_bytes={} live_bytes={} elapsed_ms={}",
            if passed { "PASS" } else { "FAIL" },
            raw.len(),
            settled.len(),
            live.len(),
            t.elapsed().as_millis()
        );
        results.push(passed);
    }

    let passed = results.iter().filter(|&&r| r).count();
    println!("{passed} / {} additional groups passed", results.len());
    if passed != results.len() {
        std::process::exit(1);
    }
}
