//! Narrow Chat presentation projection. No general HTML/XML interpretation and
//! no mutation of literal code, tool arguments, native input or host history.
const TAGS: &[&str] = &["<MemoryCite", "</MemoryCite", "<box", "</box"];

/// Presentation correlation only: these IDs never change native text or history.
#[derive(Debug, Default, Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct Sources(std::collections::BTreeMap<String, Option<Source>>);
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct Source {
    physical: String,
    final_answer: bool,
}
#[derive(Debug, serde::Serialize)]
pub struct ProgressGroup {
    pub final_id: String,
    pub progress_ids: Vec<String>,
}
impl Sources {
    pub(crate) fn observe(&mut self, id: &str, physical: String, final_answer: bool) {
        use std::collections::btree_map::Entry;
        match self.0.entry(id.into()) {
            Entry::Vacant(entry) => {
                entry.insert(Some(Source {
                    physical,
                    final_answer,
                }));
            }
            Entry::Occupied(mut entry) => {
                // An externally reused message ID cannot prove identity. Keep
                // all content visible, including on a later exact replay.
                if let Some(old) = entry.get_mut() {
                    if old.physical == physical {
                        old.final_answer |= final_answer;
                    } else {
                        *entry.get_mut() = None;
                    }
                }
            }
        }
    }
    pub(crate) fn groups(&self) -> Vec<ProgressGroup> {
        let mut physical = std::collections::BTreeMap::<_, (Vec<_>, Vec<_>)>::new();
        for (id, source) in &self.0 {
            if let Some(source) = source {
                let (finals, progress) = physical.entry(&source.physical).or_default();
                if source.final_answer {
                    finals.push(id);
                } else {
                    progress.push(id);
                }
            }
        }
        physical
            .into_values()
            .filter_map(|(finals, progress)| {
                // An ambiguous physical final offers no folding authority.
                (finals.len() == 1 && !progress.is_empty()).then(|| ProgressGroup {
                    final_id: finals[0].clone(),
                    progress_ids: progress.into_iter().cloned().collect(),
                })
            })
            .collect()
    }
}

fn tag_end(text: &str) -> Option<usize> {
    let mut quote = None;
    for (i, c) in text.char_indices().take_while(|(i, _)| *i < 4096) {
        match (quote, c) {
            (Some(q), c) if c == q => quote = None,
            (None, '\'' | '"') => quote = Some(c),
            (None, '>') => return Some(i + 1),
            _ => (),
        }
    }
    None
}

fn title(tag: &str) -> Option<&str> {
    // Only a quoted title attribute is presentation data. Other attributes are
    // ignored; their contents cannot turn into executable terminal controls.
    let mut rest = tag;
    while let Some(i) = rest.find("title=") {
        let candidate = &rest[i + 6..];
        if i > 0 && rest.as_bytes()[i - 1].is_ascii_whitespace() {
            let quote = candidate.chars().next()?;
            if matches!(quote, '\'' | '"') {
                let value = &candidate[1..];
                return value.find(quote).map(|end| &value[..end]);
            }
        }
        rest = candidate;
    }
    None
}

pub fn text(input: &str, settled: bool) -> String {
    if !input.contains('<') {
        return input.into();
    }
    let mut out = String::with_capacity(input.len());
    let mut fence = None;
    let mut inline = 0;
    let mut consumed = 0;
    let mut position = 0;
    for line in input.split_inclusive('\n') {
        let start = position;
        position += line.len();
        if consumed >= position {
            continue;
        }
        let begin = consumed.saturating_sub(start);
        let trimmed = line.trim_start_matches(' ');
        let indentation = line.len() - trimmed.len();
        let first = trimmed.chars().next().unwrap_or('\0');
        let run = trimmed.chars().take_while(|&c| c == first).count();
        if begin == 0 && indentation <= 3 && matches!(first, '`' | '~') && run >= 3 {
            match fence {
                None => fence = Some((first, run)),
                Some((c, n)) if c == first && run >= n && trimmed[run..].trim().is_empty() => {
                    fence = None
                }
                _ => (),
            }
            out.push_str(line);
            continue;
        }
        if fence.is_some() {
            out.push_str(line);
            continue;
        }
        let mut i = begin;
        while i < line.len() {
            let rest = &input[start + i..];
            let c = rest.chars().next().unwrap();
            if c == '`' {
                let n = rest.bytes().take_while(|&b| b == b'`').count();
                if inline == 0 {
                    inline = n;
                } else if inline == n {
                    inline = 0;
                }
                out.push_str(&rest[..n]);
                i += n;
                continue;
            }
            if inline == 0 && c == '<' {
                let name = TAGS.iter().find(|&&name| {
                    rest.starts_with(name)
                        && rest
                            .as_bytes()
                            .get(name.len())
                            .is_none_or(|c| c.is_ascii_whitespace() || b"/>".contains(c))
                });
                if let Some(name) = name {
                    if let Some(end) = tag_end(rest) {
                        let tag = &rest[..end];
                        if name.ends_with("box") {
                            if !out.is_empty() && !out.ends_with('\n') {
                                out.push('\n');
                            }
                            if *name == "<box" {
                                if let Some(title) = title(tag).filter(|v| !v.trim().is_empty()) {
                                    out.push_str("**");
                                    out.push_str(title);
                                    out.push_str("**\n");
                                }
                            }
                        }
                        i += end;
                        consumed = start + i;
                        continue;
                    }
                    if !settled && rest.len() <= 4096 {
                        return out;
                    }
                } else if !settled && TAGS.iter().any(|name| name.starts_with(rest)) {
                    return out;
                }
            }
            out.push(c);
            i += c.len_utf8();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_committed_identical_sources_offer_progress_folding() {
        let mut sources = Sources::default();
        sources.observe("p1", "physical-a".into(), false);
        sources.observe("p2", "physical-a".into(), false);
        sources.observe("separate-report", "physical-b".into(), false);
        assert!(sources.groups().is_empty());
        sources.observe("final", "physical-a".into(), true);
        let groups = sources.groups();
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].final_id, "final");
        assert_eq!(groups[0].progress_ids, ["p1", "p2"]);
        // Persistence/replay keeps identities; reusing an ID for another
        // physical source removes its eligibility, never its visible content.
        let mut restored: Sources =
            serde_json::from_value(serde_json::to_value(&sources).unwrap()).unwrap();
        restored.observe("p1", "another-chat-source".into(), false);
        restored.observe("p1", "physical-a".into(), false);
        assert_eq!(restored.groups()[0].progress_ids, ["p2"]);
    }
    fn streamed(input: &str, expected: &str) {
        let mut previous = String::new();
        for (i, _) in input.char_indices() {
            let next = text(&input[..i], false);
            assert!(
                next.starts_with(&previous),
                "rewrote prefix at {i}: {next:?} vs {previous:?}"
            );
            previous = next;
        }
        let final_text = text(input, true);
        assert!(final_text.starts_with(&previous));
        assert_eq!(final_text, expected);
    }
    #[test]
    fn known_components_project_without_stream_rewrites_or_another_model_turn() {
        streamed("Done<MemoryCite source=\"private > label\" />!", "Done!");
        streamed(
            "<box title=\"검증 결과\">Tests passed</box>Next",
            "**검증 결과**\nTests passed\nNext",
        );
        streamed(
            "A<box>plain <other>text</other></box>Z",
            "A\nplain <other>text</other>\nZ",
        );
        streamed(
            "<box\n title='결과'>multi\nline</box>",
            "**결과**\nmulti\nline\n",
        );
    }
    #[test]
    fn literal_code_unknown_and_malformed_markup_are_preserved() {
        for input in [
            "Use `<MemoryCite />` and ``<box>literal</box>``.",
            "```html\n<box title=\"code\">literal</box>\n```\nDone",
            "~~~\n<MemoryCite />\n~~~\nDone",
            "x < 5; <boxplot> stays",
            "unfinished <box title=\"title",
            "unknown <component value=\"x\">text</component>",
        ] {
            streamed(input, input);
        }
    }
}
