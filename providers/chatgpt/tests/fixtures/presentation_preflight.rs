//! Standalone, account-free regression oracle for the §11 presentation work.
//! Run with rustc, not as an automatically discovered Cargo integration test.
//! A failing baseline is intentional until Phase 1 fixes the renderer.
#[path = "../../src/presentation.rs"]
mod presentation;

fn all(text: &str, required: &[&str]) -> bool {
    required.iter().all(|value| text.contains(value))
}

#[cfg_attr(test, test)]
fn main() {
    let cases = [
        ("selector", r#"<segmented-control options={[{"label":"summary","value":"s"},{"label":"detail","value":"d"}]} value={view} onChange={setView}/>"#),
        ("missing_binding", r#"<grid><grid-item><caption>{metric.label}</caption><text>{metric.value}</text></grid-item></grid>"#),
        ("literal_table", "<table><table-row><table-cell>Implemented</table-cell><table-cell>6</table-cell></table-row></table>"),
        ("unknown_condition", "{#if ready}Ready{:else}Not ready{/if}"),
        ("known_condition", "{#if true}Ready{:else}Not ready{/if}"),
        ("chart_data", r#"<Chart content={{"chartType":"bar","xKey":"name","series":[{"dataKey":"count","label":"Count"}],"data":[{"name":"A","count":-2},{"name":"B","count":0},{"name":"C","count":null}]}}/>"#),
        ("nested_table", "<box>\n    | Item | Count |\n    |---|---:|\n    | Done | 6 |\n</box>"),
        ("indented_code", "Example:\n\n    <badge>literal</badge>\n    {obj.value}\n"),
        ("ordinary_markdown", "| Item | Count |\n|---|---:|\n| Done | 6 |\n"),
        ("fenced_code", "Example:\n```html\n<badge>literal</badge>\n```\nAfter code."),
        ("inline_code_then_text", "Use `Vec<T>` and continue streaming this sentence."),
        ("ordinary_comparison", "The value is x < 5. Continue streaming this sentence."),
        ("literal_braces", "The formula is {name}. Continue streaming this sentence."),
        ("link_unresolved", "<Link url={item.url} title=\"Details\"/>"),
        ("display_button", "<button onClick={() => setView(1)}>Details</button>"),
        ("cjk_unicode", "검증 ✅ <badge color=\"success\">완료</badge> 다음 항목"),
        ("row_prefix", "Start <row><badge>Ready</badge><text>Now</text></row> done"),
        ("nested_fence", "> Example:\n> ```html\n> <badge>literal</badge>\n> ```\n\nAfter quote."),
    ];
    let mut failures = 0;
    for (name, input) in cases {
        let final_text = presentation::render_for_codex_tui(input);
        let live = presentation::render_stream_prefix(input);
        let lower = final_text.to_ascii_lowercase();
        // Test semantic outcomes independently of the existing implementation.
        // Explicit English "unknown"/"unavailable"/"static" markers are the
        // fixture convention; localized production wording can be tested separately.
        let semantics = match name {
            "selector" => {
                all(&final_text, &["summary", "detail"])
                    && lower.contains("unknown")
                    && !final_text.contains("<segmented-control")
            }
            "missing_binding" => {
                all(&final_text, &["metric.label", "metric.value"]) && lower.contains("unavailable")
            }
            "literal_table" => {
                all(&final_text, &["Implemented", "6", "|"]) && !final_text.contains("<table")
            }
            "unknown_condition" => {
                all(&final_text, &["Ready", "Not ready"])
                    && lower.contains("unknown")
                    && final_text != "ReadyNot ready"
            }
            "known_condition" => final_text.contains("Ready") && !final_text.contains("Not ready"),
            "chart_data" => {
                all(&final_text, &["Count", "| A | -2", "| B | 0", "| C |"])
                    && (lower.contains("null")
                        || lower.contains("missing")
                        || lower.contains("unavailable"))
                    && !final_text.contains("<Chart")
            }
            "nested_table" => {
                all(
                    &final_text,
                    &["| Item | Count |", "|---|---:|", "| Done | 6 |"],
                ) && !final_text.lines().any(|line| line.starts_with("    |"))
            }
            "indented_code"
            | "ordinary_markdown"
            | "fenced_code"
            | "inline_code_then_text"
            | "ordinary_comparison"
            | "literal_braces"
            | "nested_fence" => final_text == input,
            "link_unresolved" => {
                final_text.contains("Details")
                    && lower.contains("unavailable")
                    && !final_text.contains("](item.url)")
            }
            "display_button" => {
                final_text.contains("Details")
                    && lower.contains("static")
                    && !final_text.contains("onClick")
            }
            "cjk_unicode" => {
                all(&final_text, &["검증 ✅", "완료", "다음 항목"])
                    && !final_text.contains("<badge")
            }
            "row_prefix" => {
                all(&final_text, &["Start", "Ready", "Now", "done"])
                    && !final_text.contains("]**Now")
                    && !final_text.contains("ReadyNow")
            }
            _ => unreachable!(),
        };
        // Check each UTF-8 snapshot, not just the final render. Every public
        // prefix must extend the previous one AND be a prefix of settled text.
        let mut previous = String::new();
        let unstable = input
            .char_indices()
            .map(|(i, _)| i)
            .chain(std::iter::once(input.len()))
            .find(|&i| {
                let rendered = presentation::render_stream_prefix(&input[..i]);
                let bad = !rendered.starts_with(&previous) || !final_text.starts_with(&rendered);
                previous = rendered;
                bad
            });
        // Once a complete ordinary Markdown construct has arrived, subsequent
        // prose must not wait for upstream settlement. No forced immediate
        // publication of unresolved DIL is required.
        let live_required = matches!(
            name,
            "ordinary_markdown"
                | "fenced_code"
                | "inline_code_then_text"
                | "ordinary_comparison"
                | "literal_braces"
                | "nested_fence"
        );
        let passed = semantics && unstable.is_none() && (!live_required || live == final_text);
        failures += usize::from(!passed);
        println!(
            "{} {name}: semantics={semantics}, unstable_at={unstable:?}, live_complete={}",
            if passed { "PASS" } else { "FAIL" },
            live == final_text
        );
        if !passed {
            println!("  settled={final_text:?}\n  live={live:?}");
        }
    }
    println!("{} / {} cases passed", cases.len() - failures, cases.len());
    if failures != 0 {
        std::process::exit(1);
    }
}
