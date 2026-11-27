//! ChatGPT Desktop model catalog normalization, Codex `model-catalog.json`
//! generation (`1M` context / `95%` effective / `800k` auto-compact), and
//! reasoning-effort variant routing.

use crate::config::{ensure_private_dir, random_hex, ModelConfig};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::sync::{Arc, RwLock};

pub type LiveModelSet = Arc<RwLock<HashSet<String>>>;

const DEFAULT_CONTEXT_WINDOW: u64 = 1_000_000;
const AUTO_COMPACT_WINDOW_PERCENT: u64 = 80;

pub fn new_live_model_set<I>(initial: I, catalog_path: Option<&Path>) -> LiveModelSet
where
    I: IntoIterator<Item = String>,
{
    let mut slugs: HashSet<String> = initial
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect();
    if let Some(path) = catalog_path {
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(doc) = serde_json::from_str::<Value>(&raw) {
                if let Some(arr) = doc.get("models").and_then(Value::as_array) {
                    for entry in arr {
                        if let Some(slug) = entry.get("slug").and_then(Value::as_str) {
                            let trimmed = slug.trim();
                            if !trimmed.is_empty() {
                                slugs.insert(trimmed.to_owned());
                            }
                        }
                    }
                }
            }
        }
    }
    Arc::new(RwLock::new(slugs))
}

fn update_live_models(set: Option<&LiveModelSet>, models: &[CatalogModel]) {
    let Some(lock) = set else {
        return;
    };
    let fresh: HashSet<String> = models
        .iter()
        .map(|m| m.slug.trim())
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if fresh.is_empty() {
        return;
    }
    if let Ok(mut guard) = lock.write() {
        *guard = fresh;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatalogModel {
    pub slug: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub max_tokens: Option<u64>,
    pub tags: Vec<String>,
}

pub fn catalog_models_from_status(st: &Value) -> Vec<CatalogModel> {
    let mut out = Vec::new();
    if let Some(arr) = st.get("models").and_then(Value::as_array) {
        for m in arr {
            let Some(slug) = m
                .get("slug")
                .and_then(Value::as_str)
                .or_else(|| m.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
            else {
                continue;
            };
            let title = m
                .get("title")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let description = m
                .get("description")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(ToOwned::to_owned);
            let max_tokens = m
                .get("max_tokens")
                .and_then(Value::as_u64)
                .filter(|&n| n > 0);
            let tags = m
                .get("tags")
                .and_then(Value::as_array)
                .map(|arr| {
                    arr.iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToOwned::to_owned)
                        .collect()
                })
                .unwrap_or_default();
            out.push(CatalogModel {
                slug: slug.to_owned(),
                title,
                description,
                max_tokens,
                tags,
            });
        }
    }
    out
}

pub fn fallback_catalog_models(model_cfg: &ModelConfig) -> Vec<CatalogModel> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let candidates = std::iter::once(model_cfg.default.as_str())
        .chain(model_cfg.available.iter().map(String::as_str));
    for slug in candidates {
        let trimmed = slug.trim();
        if trimmed.is_empty() || !seen.insert(trimmed.to_owned()) {
            continue;
        }
        out.push(CatalogModel {
            slug: trimmed.to_owned(),
            title: None,
            description: None,
            max_tokens: None,
            tags: Vec::new(),
        });
    }
    out
}

fn is_codex_only_slug(slug: &str) -> bool {
    let lower = slug.to_ascii_lowercase();
    lower.ends_with("-wm") || lower.contains("-codex") || lower.starts_with("codex-")
}

fn is_hidden_catalog_slug(slug: &str, tags: &[String]) -> bool {
    let lower = slug.to_ascii_lowercase();
    tags.iter().any(|t| t.eq_ignore_ascii_case("hidden"))
        || lower.contains("reserved_gate")
        || lower.starts_with("chatgpt_alpha_model_external_access_")
        || lower == "auto"
        || is_codex_only_slug(slug)
}

fn canonical_family_slug(slug: &str) -> String {
    let lower = slug.trim().to_ascii_lowercase();
    if let Some(base) = lower.strip_suffix("-instant") {
        return base.to_owned();
    }
    if let Some(base) = lower.strip_suffix("-thinking") {
        return base.to_owned();
    }
    if let Some(base) = lower.strip_suffix("-t-mini") {
        return format!("{base}-mini");
    }
    lower
}

struct ParsedGptFamily {
    major: u32,
    minor: u32,
    patch: u32,
    minor_present: bool,
    tier: Option<String>,
}

fn parse_gpt_family(family_slug: &str) -> Option<ParsedGptFamily> {
    let rest = family_slug.strip_prefix("gpt-")?;
    let mut nums: Vec<u32> = Vec::new();
    let mut tier_parts: Vec<&str> = Vec::new();
    let mut in_tier = false;
    for part in rest.split('-') {
        if !in_tier {
            let parsed: Option<Vec<u32>> = if part.is_empty() {
                None
            } else {
                part.split('.')
                    .map(|seg| {
                        if !seg.is_empty() && seg.bytes().all(|b| b.is_ascii_digit()) {
                            seg.parse::<u32>().ok()
                        } else {
                            None
                        }
                    })
                    .collect()
            };
            if let Some(seg_nums) = parsed {
                nums.extend(seg_nums);
                continue;
            }
            in_tier = true;
        }
        if !part.is_empty() {
            tier_parts.push(part);
        }
    }
    let &major = nums.first()?;
    let minor_present = nums.len() > 1;
    let minor = nums.get(1).copied().unwrap_or(0);
    let patch = nums.get(2).copied().unwrap_or(0);
    let tier = if tier_parts.is_empty() {
        None
    } else {
        Some(tier_parts.join("-"))
    };
    Some(ParsedGptFamily {
        major,
        minor,
        patch,
        minor_present,
        tier,
    })
}

fn capitalize_word(word: &str) -> String {
    let mut chars = word.chars();
    match chars.next() {
        Some(first) => format!("{}{}", first.to_ascii_uppercase(), chars.as_str()),
        None => String::new(),
    }
}

fn canonical_display_name(family_slug: &str, raw_title: Option<&str>) -> String {
    if let Some(parsed) = parse_gpt_family(family_slug) {
        let mut base = if parsed.patch > 0 {
            format!("GPT-{}.{}.{}", parsed.major, parsed.minor, parsed.patch)
        } else if parsed.minor_present {
            format!("GPT-{}.{}", parsed.major, parsed.minor)
        } else {
            format!("GPT-{}", parsed.major)
        };
        if let Some(tier) = parsed.tier.as_deref() {
            match tier {
                "pro" => base.push_str(" Pro"),
                "mini" => base.push_str(" Mini"),
                other => {
                    for word in other.split('-') {
                        if !word.is_empty() {
                            base.push(' ');
                            base.push_str(&capitalize_word(word));
                        }
                    }
                }
            }
        }
        return base;
    }
    raw_title
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| family_slug.to_owned())
}

fn family_sort_key(
    family_slug: &str,
) -> (
    u8,
    std::cmp::Reverse<u32>,
    std::cmp::Reverse<u32>,
    std::cmp::Reverse<u32>,
    u8,
    &str,
) {
    use std::cmp::Reverse;
    if let Some(parsed) = parse_gpt_family(family_slug) {
        let tier_rank = match parsed.tier.as_deref() {
            None => 0,
            Some("pro") => 1,
            Some("mini") => 2,
            Some(_) => 3,
        };
        (
            0,
            Reverse(parsed.major),
            Reverse(parsed.minor),
            Reverse(parsed.patch),
            tier_rank,
            family_slug,
        )
    } else {
        (1, Reverse(0), Reverse(0), Reverse(0), 0, family_slug)
    }
}

fn is_non_thinking_slug(slug: &str) -> bool {
    let lower = slug.to_ascii_lowercase();
    lower.contains("instant") || lower == "research"
}

fn normalize_catalog_default_effort(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "none" | "zero" => "none",
        "minimal" | "min" | "low" => "low",
        "high" | "extended" => "high",
        "xhigh" => "xhigh",
        "max" | "ultra" => "max",
        _ => "medium",
    }
}

fn standard_reasoning_levels(include_none: bool) -> Value {
    let mut levels = Vec::new();
    if include_none {
        levels.push(
            json!({ "effort": "none", "description": "Direct response without extended thinking" }),
        );
    }
    levels.extend([
        json!({ "effort": "low", "description": "Lighter reasoning for fast replies" }),
        json!({ "effort": "medium", "description": "Balanced speed and reasoning depth" }),
        json!({ "effort": "high", "description": "Greater reasoning depth for complex tasks" }),
        json!({ "effort": "xhigh", "description": "Extra high reasoning depth for hard tasks" }),
        json!({ "effort": "max", "description": "Maximum ChatGPT reasoning depth" }),
    ]);
    Value::Array(levels)
}

struct CatalogEntrySpec<'a> {
    slug: &'a str,
    display_name: &'a str,
    description: &'a str,
    default_effort: &'a str,
    supported_efforts: Value,
    visibility: &'a str,
    priority: i64,
    max_tokens: Option<u64>,
}

fn make_catalog_entry(spec: CatalogEntrySpec<'_>) -> Value {
    let context_window = spec.max_tokens.unwrap_or(0).max(DEFAULT_CONTEXT_WINDOW);
    let auto_compact_token_limit = context_window * AUTO_COMPACT_WINDOW_PERCENT / 100;
    json!({
        "slug": spec.slug,
        "display_name": spec.display_name,
        "description": spec.description,
        "default_reasoning_level": spec.default_effort,
        "supported_reasoning_levels": spec.supported_efforts,
        "shell_type": "unified_exec",
        "visibility": spec.visibility,
        "supported_in_api": true,
        "priority": spec.priority,
        "base_instructions": "",
        "supports_reasoning_summaries": true,
        "supports_reasoning_summary_parameter": true,
        "default_reasoning_summary": "auto",
        "support_verbosity": false,
        "truncation_policy": { "mode": "tokens", "limit": 10000 },
        "context_window": context_window,
        "effective_context_window_percent": 95,
        "auto_compact_token_limit": auto_compact_token_limit,
        "input_modalities": ["text", "image"],
        "experimental_supported_tools": []
    })
}

pub fn resolve_dispatch_model_and_effort(
    requested_model: &str,
    requested_effort: Option<&str>,
    known_slugs: &HashSet<String>,
) -> (String, Option<String>) {
    let model = requested_model.trim();
    let lower = model.to_ascii_lowercase();
    let clean_effort = requested_effort
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned);

    if lower.ends_with("-instant")
        || lower.ends_with("-thinking")
        || lower.ends_with("-t-mini")
        || lower.ends_with("-pro")
        || lower.ends_with("-wm")
        || matches!(lower.as_str(), "auto" | "research")
    {
        return (model.to_owned(), clean_effort);
    }

    let effort_key = clean_effort.as_deref().map(str::to_ascii_lowercase);
    let is_none = matches!(effort_key.as_deref(), Some("none" | "zero"));
    let is_thinking = matches!(
        effort_key.as_deref(),
        Some(
            "minimal"
                | "min"
                | "low"
                | "medium"
                | "standard"
                | "high"
                | "extended"
                | "xhigh"
                | "max"
                | "ultra"
        )
    );

    if let Some(prefix) = lower.strip_suffix("-mini") {
        if is_none {
            let target = if known_slugs.is_empty() || known_slugs.contains(&lower) {
                model.to_owned()
            } else {
                let fallback = format!("{prefix}-t-mini");
                if known_slugs.contains(&fallback) {
                    fallback
                } else {
                    model.to_owned()
                }
            };
            return (target, None);
        }
        if is_thinking {
            let candidate = format!("{prefix}-t-mini");
            if known_slugs.contains(&candidate) {
                return (candidate, clean_effort);
            }
        }
        return (model.to_owned(), clean_effort);
    }

    if is_none {
        let candidate = format!("{lower}-instant");
        if known_slugs.contains(&candidate) {
            return (candidate, None);
        }
        return (model.to_owned(), clean_effort);
    }

    if is_thinking {
        let candidate = format!("{lower}-thinking");
        if known_slugs.contains(&candidate) {
            return (candidate, clean_effort);
        }
    }

    (model.to_owned(), clean_effort)
}

pub fn build_codex_model_catalog(
    models: &[CatalogModel],
    model_cfg: &ModelConfig,
    parallel_tool_calls: bool,
) -> Value {
    struct FamilyGroup {
        title: Option<String>,
        description: Option<String>,
        max_tokens: Option<u64>,
        has_non_instant: bool,
        has_bare_or_base: bool,
    }

    let mut families: BTreeMap<String, FamilyGroup> = BTreeMap::new();
    let mut raw_models: Vec<CatalogModel> = Vec::new();
    let mut seen_raw = HashSet::new();

    for m in models {
        let slug = m.slug.trim();
        if slug.is_empty() || !seen_raw.insert(slug.to_owned()) {
            continue;
        }
        raw_models.push(CatalogModel {
            slug: slug.to_owned(),
            title: m.title.clone(),
            description: m.description.clone(),
            max_tokens: m.max_tokens,
            tags: m.tags.clone(),
        });

        if is_hidden_catalog_slug(slug, &m.tags) {
            continue;
        }
        let family = canonical_family_slug(slug);
        let non_instant = !is_non_thinking_slug(slug);
        let group = families
            .entry(family.clone())
            .or_insert_with(|| FamilyGroup {
                title: m.title.clone(),
                description: m.description.clone(),
                max_tokens: m.max_tokens,
                has_non_instant: non_instant,
                has_bare_or_base: slug.eq_ignore_ascii_case(&family),
            });
        if group.title.is_none() && m.title.is_some() {
            group.title = m.title.clone();
        }
        if group.description.is_none() && m.description.is_some() {
            group.description = m.description.clone();
        }
        group.max_tokens = match (group.max_tokens, m.max_tokens) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        };
        group.has_non_instant |= non_instant;
        if slug.eq_ignore_ascii_case(&family) {
            group.has_bare_or_base = true;
            if m.description.is_some() {
                group.description = m.description.clone();
            }
        }
    }

    let mut sorted_families: Vec<(String, FamilyGroup)> = families.into_iter().collect();
    sorted_families.sort_by(|a, b| family_sort_key(&a.0).cmp(&family_sort_key(&b.0)));

    let default_effort = normalize_catalog_default_effort(&model_cfg.reasoning_effort);
    let mut entries = Vec::new();
    let mut emitted_slugs = HashSet::new();

    for (family_slug, group) in &sorted_families {
        let priority = (entries.len() + 1) as i64;
        let display_name = canonical_display_name(family_slug, group.title.as_deref());
        let description = group
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("ChatGPT Desktop model ({family_slug})"));
        let is_pro = family_slug.ends_with("-pro");
        let (eff_default, supported_efforts) =
            if !group.has_non_instant || family_slug == "research" {
                ("none", json!([]))
            } else if is_pro {
                let eff = if default_effort == "none" {
                    "medium"
                } else {
                    default_effort
                };
                (eff, standard_reasoning_levels(false))
            } else {
                (default_effort, standard_reasoning_levels(true))
            };
        emitted_slugs.insert(family_slug.clone());
        entries.push(make_catalog_entry(CatalogEntrySpec {
            slug: family_slug,
            display_name: &display_name,
            description: &description,
            default_effort: eff_default,
            supported_efforts,
            visibility: "list",
            priority,
            max_tokens: group.max_tokens,
        }));
    }

    let mut hidden_idx = 0i64;
    for m in &raw_models {
        let lower = m.slug.to_ascii_lowercase();
        if emitted_slugs.contains(&lower) || !emitted_slugs.insert(m.slug.clone()) {
            continue;
        }
        hidden_idx += 1;
        let display_name = m
            .title
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or(&m.slug);
        let description = m
            .description
            .as_deref()
            .filter(|s| !s.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("ChatGPT Desktop model ({})", m.slug));
        let (eff_default, supported_efforts) = if is_non_thinking_slug(&m.slug) {
            ("none", json!([]))
        } else if lower.ends_with("-pro") {
            let eff = if default_effort == "none" {
                "medium"
            } else {
                default_effort
            };
            (eff, standard_reasoning_levels(false))
        } else {
            (default_effort, standard_reasoning_levels(true))
        };
        entries.push(make_catalog_entry(CatalogEntrySpec {
            slug: &m.slug,
            display_name,
            description: &description,
            default_effort: eff_default,
            supported_efforts,
            visibility: "hide",
            priority: 100 + hidden_idx,
            max_tokens: m.max_tokens,
        }));
    }

    let mut catalog = json!({ "models": entries });
    set_catalog_parallel_tool_calls(&mut catalog, parallel_tool_calls);
    catalog
}

pub fn set_catalog_parallel_tool_calls(catalog: &mut Value, supported: bool) {
    if let Some(models) = catalog.get_mut("models").and_then(Value::as_array_mut) {
        for model in models.iter_mut().filter_map(Value::as_object_mut) {
            model.insert("supports_parallel_tool_calls".into(), json!(supported));
        }
    }
}

pub fn sync_model_catalog_file(path: &Path, catalog: &Value) -> io::Result<()> {
    let mut encoded = serde_json::to_vec_pretty(catalog)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    encoded.push(b'\n');
    if let Ok(existing) = fs::read(path) {
        if existing == encoded {
            return Ok(());
        }
    }
    if let Some(parent) = path.parent() {
        ensure_private_dir(parent).map_err(|err| io::Error::other(err.to_string()))?;
    }
    let tmp = path.with_extension(format!(
        "json.tmp-{}-{}",
        std::process::id(),
        random_hex(4).map_err(|err| io::Error::other(err.to_string()))?
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .custom_flags(libc::O_NOFOLLOW)
        .open(&tmp)?;
    let write_res = file.write_all(&encoded).and_then(|()| file.sync_all());
    drop(file);
    if let Err(err) = write_res {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    if let Err(err) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(err);
    }
    Ok(())
}

pub fn sync_catalog_from_status(
    catalog_path: Option<&Path>,
    st: &Value,
    model_cfg: &ModelConfig,
    live_models: Option<&LiveModelSet>,
    parallel_tool_calls: bool,
) {
    if st.get("host_running").and_then(Value::as_bool) == Some(false) {
        return;
    }
    let models = catalog_models_from_status(st);
    if models.is_empty() {
        return;
    }
    update_live_models(live_models, &models);
    let Some(path) = catalog_path else {
        return;
    };
    let catalog = build_codex_model_catalog(&models, model_cfg, parallel_tool_calls);
    if let Err(err) = sync_model_catalog_file(path, &catalog) {
        eprintln!(
            "chatgpt-provider: warning: failed to update {}: {err}",
            path.display()
        );
    }
}
