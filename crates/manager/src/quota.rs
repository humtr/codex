//! Account usage and rotation policy. Optional Manager policy, never Core state.
use super::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::os::unix::fs::MetadataExt;
use std::time::{SystemTime, UNIX_EPOCH};

const FRESH: u64 = 90;
const MAX_BYTES: u64 = 1024 * 1024;
const ERR_QUOTA: ManagerError = ManagerError::operation(
    "codex termux: usage or rotation state is unavailable or invalid; no automatic handoff",
);

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Window {
    minutes: i64,
    remaining: i64,
    reset: Option<u64>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Bucket {
    id: String,
    model: Option<String>,
    windows: Vec<Window>,
    blocked: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Usage {
    schema: u32,
    home: [u64; 2],
    auth: Vec<i64>,
    observed: u64,
    status: String,
    account: Option<String>,
    allowed: Option<bool>,
    buckets: Vec<Bucket>,
    models: Vec<String>,
    #[serde(default)]
    efforts: BTreeMap<String, Vec<String>>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct Member {
    home: [u64; 2],
    account: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
struct Policy {
    enabled: bool,
    policy: String,
    accounts: BTreeMap<String, Member>,
    order: Vec<String>,
    reserve: i64,
    proactive: i64,
    cooldown: u64,
    weights: [i64; 3],
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Handoff {
    profile: String,
    thread: String,
    turn: String,
    model: String,
    trigger: String,
    flags: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Attempt {
    turn: String,
    at: u64,
    source: String,
    target: String,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            enabled: false,
            policy: "combined".into(),
            accounts: BTreeMap::new(),
            order: Vec::new(),
            reserve: 5,
            proactive: 0,
            cooldown: 300,
            weights: [1, 1, 1],
        }
    }
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn identity(home: &Path) -> Result<[u64; 2], ManagerError> {
    let m = fs::symlink_metadata(home).map_err(|_| ERR_QUOTA)?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o7777 != 0o700 {
        return Err(ERR_QUOTA);
    }
    Ok([m.dev(), m.ino()])
}
fn auth_stamp(home: &Path) -> Result<Vec<i64>, ManagerError> {
    let path = home.join("auth.json");
    if inspect_path(&path)? == PathPresence::Missing {
        return Ok(Vec::new());
    }
    let m = fs::symlink_metadata(path).map_err(|_| ERR_QUOTA)?;
    if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o077 != 0 {
        return Err(ERR_QUOTA);
    }
    Ok(vec![
        m.dev() as i64,
        m.ino() as i64,
        m.len() as i64,
        m.mtime(),
        m.mtime_nsec(),
    ])
}
fn home_for(context: &Context, id: &str) -> Result<PathBuf, ManagerError> {
    let target = parse_target(&OsString::from(id))?;
    if profile::target_name(&target) != id {
        return Err(ERR_PROFILE);
    }
    profile::validate_target(context, &target)?;
    Ok(match target {
        ProfileTarget::Default => context.home.join(".codex"),
        ProfileTarget::Custom(id) => profile_home_path(
            &existing_manager_profiles(context)?.ok_or(ERR_PROFILE)?,
            &id,
        ),
    })
}
fn cache(context: &Context) -> PathBuf {
    context.home.join(".cache/codex/manager/usage")
}
fn read_json<T: for<'a> Deserialize<'a>>(path: &Path) -> Result<Option<T>, ManagerError> {
    if inspect_path(path)? == PathPresence::Missing {
        return Ok(None);
    }
    let mut f = OpenOptions::new()
        .read(true)
        .custom_flags(libc::O_NOFOLLOW)
        .open(path)
        .map_err(|_| ERR_QUOTA)?;
    let m = f.metadata().map_err(|_| ERR_QUOTA)?;
    if !m.is_file()
        || m.uid() != unsafe { libc::geteuid() }
        || m.mode() & 0o7777 != 0o600
        || m.len() > MAX_BYTES
    {
        return Err(ERR_QUOTA);
    }
    let mut bytes = Vec::new();
    Read::by_ref(&mut f)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| ERR_QUOTA)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ERR_QUOTA);
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| ERR_QUOTA)
}
fn publish<T: Serialize>(path: &Path, value: &T) -> Result<(), ManagerError> {
    let bytes = serde_json::to_vec(value).map_err(|_| ERR_QUOTA)?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err(ERR_QUOTA);
    }
    if inspect_path(path)? == PathPresence::Present {
        let _: Option<Value> = read_json(path)?;
    }
    let parent = path.parent().ok_or(ERR_QUOTA)?;
    ensure_private_directory(parent)?;
    let temp = parent.join(format!(
        ".usage-{}-{}",
        std::process::id(),
        TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    let result = (|| {
        let mut f = OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|_| ERR_QUOTA)?;
        f.write_all(&bytes).map_err(|_| ERR_QUOTA)?;
        f.sync_all().map_err(|_| ERR_QUOTA)?;
        fs::rename(&temp, path).map_err(|_| ERR_QUOTA)?;
        sync_directory(parent)
    })();
    if result.is_err() {
        let _ = fs::remove_file(temp);
    }
    result
}
fn policy_path(context: &Context) -> PathBuf {
    manager_base(context).join("manager/rotation-v1.json")
}
fn policy(context: &Context) -> Result<Policy, ManagerError> {
    let value: Policy = read_json(&policy_path(context))?.unwrap_or_default();
    validate_policy(&value)?;
    Ok(value)
}
fn validate_policy(value: &Policy) -> Result<(), ManagerError> {
    if !matches!(
        value.policy.as_str(),
        "5h" | "weekly" | "combined" | "remaining" | "custom"
    ) || !(0..100).contains(&value.reserve)
        || !(0..=100).contains(&value.proactive)
        || !(30..=86400).contains(&value.cooldown)
        || value.weights.iter().any(|x| !(0..=1000).contains(x))
        || value.accounts.len() > 256
        || value.order.len() > 256
    {
        return Err(ERR_QUOTA);
    }
    let mut seen = BTreeSet::new();
    if value
        .order
        .iter()
        .any(|id| !seen.insert(id) || !value.accounts.contains_key(id))
    {
        return Err(ERR_QUOTA);
    }
    Ok(())
}
fn load(context: &Context, id: &str) -> Option<Usage> {
    let home = home_for(context, id).ok()?;
    let value: Usage = read_json(&cache(context).join(format!("{id}.json"))).ok()??;
    (value.schema == 1
        && value.home == identity(&home).ok()?
        && value.auth == auth_stamp(&home).ok()?)
    .then_some(value)
}
fn fresh(value: &Usage, time: u64) -> bool {
    value.status == "ready" && time >= value.observed && time - value.observed <= FRESH
}
fn nonempty(value: &Value) -> Option<String> {
    value
        .as_str()
        .filter(|s| !s.is_empty() && s.len() <= 256 && !s.chars().any(char::is_control))
        .map(str::to_owned)
}
fn bucket(value: &Value, id: &str) -> Result<Bucket, ManagerError> {
    let mut windows = Vec::new();
    for key in ["primary", "secondary"] {
        if value[key].is_null() {
            continue;
        }
        let v = &value[key];
        let remaining = 100
            - v["usedPercent"]
                .as_i64()
                .filter(|x| (0..=100).contains(x))
                .ok_or(ERR_QUOTA)?;
        if let Some(minutes) = v["windowDurationMins"].as_i64().filter(|x| *x > 0) {
            windows.push(Window {
                minutes,
                remaining,
                reset: v["resetsAt"].as_u64(),
            });
        }
    }
    if windows.len() == 2 && windows[0].minutes == windows[1].minutes {
        return Err(ERR_QUOTA);
    }
    Ok(Bucket {
        id: id.to_owned(),
        model: nonempty(&value["normalModelSlug"]),
        windows,
        blocked: value["spendControlReached"].as_bool() == Some(true)
            || matches!(
                value["rateLimitReachedType"].as_str(),
                Some(
                    "workspace_owner_credits_depleted"
                        | "workspace_member_credits_depleted"
                        | "workspace_owner_usage_limit_reached"
                        | "workspace_member_usage_limit_reached"
                )
            ),
    })
}
fn query(context: &Context, id: &str, deadline: Instant) -> Result<Usage, ManagerError> {
    let home = home_for(context, id)?;
    let binding = identity(&home)?;
    let auth = auth_stamp(&home)?;
    let mut client = quota_rpc::Client::new(context, &home, deadline)?;
    let account = client.call("account/read", json!({"refreshToken":false}))?;
    let mut result = Usage {
        schema: 1,
        home: binding,
        auth: auth.clone(),
        observed: now(),
        status: "signed_out".into(),
        account: None,
        allowed: None,
        buckets: Vec::new(),
        models: Vec::new(),
        efforts: BTreeMap::new(),
    };
    if account["account"]["type"].as_str() != Some("chatgpt") {
        result.status = if account["account"].is_null() {
            "signed_out"
        } else {
            "api_key"
        }
        .into();
        return Ok(result);
    }
    let limits = client.call(
        "account/rateLimits/read",
        json!({"supportsLunaReserve":false,"excludeResetCreditDetails":true}),
    )?;
    result.account = nonempty(&limits["accountId"])
        .or_else(|| nonempty(&account["workspaceRouting"]["chatgptAccountId"]));
    if let (Some(a), Some(b)) = (
        nonempty(&limits["accountId"]),
        nonempty(&account["workspaceRouting"]["chatgptAccountId"]),
    ) {
        if a != b {
            return Err(ERR_QUOTA);
        }
    }
    result.allowed = limits["ordinaryUsageAllowed"].as_bool();
    let mut buckets = BTreeMap::new();
    if let Some(id) = nonempty(&limits["rateLimits"]["limitId"]) {
        buckets.insert(id.clone(), bucket(&limits["rateLimits"], &id)?);
    }
    if let Some(map) = limits["rateLimitsByLimitId"].as_object() {
        if map.len() > 64 {
            return Err(ERR_QUOTA);
        }
        for (id, value) in map {
            if nonempty(&json!(id)).is_none() {
                return Err(ERR_QUOTA);
            }
            buckets.insert(id.clone(), bucket(value, id)?);
        }
    }
    result.buckets = buckets.into_values().collect();
    let mut cursor = Value::Null;
    let mut cursors = BTreeSet::new();
    loop {
        let models = client.call(
            "model/list",
            json!({"limit":100,"cursor":cursor,"includeHidden":false}),
        )?;
        for model in models["data"].as_array().ok_or(ERR_QUOTA)? {
            if let Some(id) = nonempty(&model["model"]) {
                let efforts = model["supportedReasoningEfforts"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter_map(|v| nonempty(&v["reasoningEffort"]))
                    .take(32)
                    .collect();
                result.efforts.insert(id.clone(), efforts);
                result.models.push(id);
            }
        }
        if result.models.len() > 256 {
            return Err(ERR_QUOTA);
        }
        if models["nextCursor"].is_null() {
            break;
        }
        let next = nonempty(&models["nextCursor"]).ok_or(ERR_QUOTA)?;
        if !cursors.insert(next.clone()) {
            return Err(ERR_QUOTA);
        }
        cursor = json!(next);
    }
    if binding != identity(&home)? || auth != auth_stamp(&home)? {
        return Err(ERR_QUOTA);
    }
    result.status = "ready".into();
    result.observed = now();
    Ok(result)
}
fn refresh(context: &Context, ids: &[String]) -> Result<(), ManagerError> {
    ensure_private_directory(&cache(context))?;
    let deadline = Instant::now() + Duration::from_secs(20);
    let _lock = match profile::directory_lock(&cache(context), libc::LOCK_EX) {
        Ok(lock) => lock,
        Err(error) if error == profile::ERR_BUSY => {
            // Reuse the in-flight refresh, rather than opening another set of
            // account clients or returning before its fresh rows are available.
            while Instant::now() < deadline {
                if profile::directory_lock(&cache(context), libc::LOCK_SH).is_ok() {
                    return Ok(());
                }
                thread::sleep(Duration::from_millis(25));
            }
            return Ok(());
        }
        Err(error) => return Err(error),
    };
    // Four bounded readers; only this explicit operation creates them.
    thread::scope(|scope| {
        for part in ids.chunks(ids.len().div_ceil(4).max(1)) {
            scope.spawn(move || {
                for id in part {
                    if Instant::now() >= deadline {
                        break;
                    }
                    if load(context, id).is_some_and(|u| now().saturating_sub(u.observed) < 5) {
                        continue;
                    }
                    let value = query(
                        context,
                        id,
                        (Instant::now() + Duration::from_secs(8)).min(deadline),
                    );
                    match value {
                        Ok(value) => {
                            let _ = publish(&cache(context).join(format!("{id}.json")), &value);
                        }
                        Err(_) => {
                            if let Ok(home) = home_for(context, id) {
                                if let (Ok(home), Ok(auth)) = (identity(&home), auth_stamp(&home)) {
                                    let value = Usage {
                                        schema: 1,
                                        home,
                                        auth,
                                        observed: now(),
                                        status: "unavailable".into(),
                                        account: None,
                                        allowed: None,
                                        buckets: Vec::new(),
                                        models: Vec::new(),
                                        efforts: BTreeMap::new(),
                                    };
                                    let _ =
                                        publish(&cache(context).join(format!("{id}.json")), &value);
                                }
                            }
                        }
                    }
                }
            });
        }
    });
    // A first Core launch may contend with another account's shared-layout
    // preparation. Retry failed reads once after the parallel readers settle.
    // This never retries inference or touches an existing server/job.
    for id in ids {
        if Instant::now() >= deadline {
            break;
        }
        if load(context, id).is_some_and(|u| u.status == "unavailable") {
            if let Ok(value) = query(
                context,
                id,
                (Instant::now() + Duration::from_secs(8)).min(deadline),
            ) {
                let _ = publish(&cache(context).join(format!("{id}.json")), &value);
            }
        }
    }
    Ok(())
}
fn selected_bucket<'a>(usage: &'a Usage, model: Option<&str>) -> Option<&'a Bucket> {
    if let Some(model) = model {
        if let Some(b) = usage
            .buckets
            .iter()
            .find(|b| b.model.as_deref() == Some(model) || b.id == model)
        {
            return Some(b);
        }
    }
    usage.buckets.iter().find(|b| b.id == "codex")
}
fn reason(
    usage: Option<&Usage>,
    member: Option<&Member>,
    model: Option<&str>,
    reserve: i64,
    time: u64,
) -> &'static str {
    let Some(u) = usage else {
        return "unknown";
    };
    if !fresh(u, time) {
        return match u.status.as_str() {
            "ready" => "stale",
            "signed_out" => "signed out",
            "api_key" => "API key (not a Codex quota pool)",
            _ => "unavailable",
        };
    }
    if u.account.is_none() {
        return "identity unknown";
    }
    if member.is_some_and(|m| m.home != u.home || Some(&m.account) != u.account.as_ref()) {
        return "account changed";
    }
    if u.allowed != Some(true) {
        return if u.allowed == Some(false) {
            "usage blocked"
        } else {
            "permission unknown"
        };
    }
    if model.is_some_and(|m| !u.models.iter().any(|id| id == m)) {
        return "model unavailable";
    }
    let Some(bucket) = selected_bucket(u, model) else {
        return "bucket unknown";
    };
    if !u.buckets.iter().any(|b| {
        b.id == "codex"
            && [300, 10080]
                .iter()
                .all(|minutes| b.windows.iter().any(|w| w.minutes == *minutes))
    }) {
        return "windows unknown";
    }
    if u.buckets
        .iter()
        .any(|b| b.id == "codex" && (b.blocked || b.windows.iter().any(|w| w.remaining <= reserve)))
    {
        return "reserve or exhausted";
    }
    if bucket.blocked {
        return "usage blocked";
    }
    if bucket.windows.is_empty() {
        return "windows unknown";
    }
    if bucket.windows.iter().any(|w| w.remaining <= reserve) {
        return "reserve or exhausted";
    }
    "ready"
}
fn score(usage: &Usage, policy: &Policy, model: Option<&str>, time: u64) -> [i64; 3] {
    let bucket = selected_bucket(usage, model).unwrap();
    let window = |minutes| bucket.windows.iter().find(|w| w.minutes == minutes);
    let reset = |minutes| {
        window(minutes)
            .and_then(|w| w.reset)
            .filter(|r| *r > time)
            .map(|r| r.saturating_sub(time).min(i64::MAX as u64) as i64)
            .unwrap_or(i64::MAX / 8)
    };
    let min = bucket
        .windows
        .iter()
        .map(|w| w.remaining)
        .min()
        .unwrap_or(0);
    match policy.policy.as_str() {
        "5h" => [reset(300), -min, reset(10080)],
        "weekly" => [reset(10080), -min, reset(300)],
        "remaining" => [-min, reset(300), reset(10080)],
        "custom" => {
            let remain = |minutes| window(minutes).map_or(0, |w| w.remaining);
            let urgency = reset(300).min(reset(10080)).min(604800) / 3600;
            [
                -(remain(300) * policy.weights[0] + remain(10080) * policy.weights[1])
                    + urgency * policy.weights[2],
                0,
                0,
            ]
        }
        _ => [
            -min,
            reset(300).max(reset(10080)),
            reset(300).min(reset(10080)),
        ],
    }
}
fn recommended(
    context: &Context,
    ids: &[String],
    policy: &Policy,
    current: Option<&str>,
    model: Option<&str>,
) -> Option<String> {
    let current_account = current
        .and_then(|id| load(context, id))
        .and_then(|u| u.account);
    let time = now();
    let cooldowns: BTreeMap<String, u64> = read_json(&cache(context).join("cooldowns.json"))
        .ok()
        .flatten()
        .unwrap_or_default();
    let mut choices = Vec::new();
    for id in ids {
        let Some(member) = policy.accounts.get(id) else {
            continue;
        };
        let usage = load(context, id);
        if current == Some(id.as_str())
            || current_account.as_ref() == Some(&member.account)
            || cooldowns
                .get(&member.account)
                .is_some_and(|until| *until > time)
            || reason(usage.as_ref(), Some(member), model, policy.reserve, time) != "ready"
        {
            continue;
        }
        let usage = usage.unwrap();
        let order = policy
            .order
            .iter()
            .position(|x| x == id)
            .unwrap_or(usize::MAX);
        let mut rank = score(&usage, policy, model, time);
        if policy.policy == "custom" {
            rank = [order.min(1_000_000) as i64, rank[0], rank[1]];
        }
        choices.push((rank, order, id.clone()));
    }
    choices.sort();
    choices.into_iter().next().map(|x| x.2)
}
fn handoff(context: &Context, value: &str) -> Result<Option<String>, ManagerError> {
    if value.len() > 65536 {
        return Err(ERR_USAGE);
    }
    let request: Handoff = serde_json::from_str(value).map_err(|_| ERR_USAGE)?;
    if !canonical_session_id(&request.thread)
        || request.turn.is_empty()
        || request.turn.len() > 512
        || request.turn.chars().any(char::is_control)
        || request.model.len() > 256
        || !matches!(request.trigger.as_str(), "quota" | "proactive")
    {
        return Err(ERR_USAGE);
    }
    profile_resume::validate_flags(&request.flags)?;
    // Own config-profile/model arguments must match the independently checked model.
    if !request
        .flags
        .chunks(2)
        .any(|p| p[0] == "--model" && p[1] == request.model)
    {
        return Err(ERR_USAGE);
    }
    let current: Value =
        serde_json::from_str(&profile_view::snapshot(context)?).map_err(|_| ERR_QUOTA)?;
    let source = current["current"].as_str().ok_or(ERR_QUOTA)?;
    if source == request.profile {
        return Err(ERR_QUOTA);
    }
    let registry = manager_profiles_for_create(context)?;
    let _registry = profile::directory_lock(&registry, libc::LOCK_SH)?;
    let policy = policy(context)?;
    if !policy.enabled {
        return Err(ERR_QUOTA);
    }
    let destination = policy.accounts.get(&request.profile).ok_or(ERR_QUOTA)?;
    let source_usage = query(context, source, Instant::now() + Duration::from_secs(8))?;
    if source_usage.account.is_none()
        || policy.accounts.get(source).is_some_and(|member| {
            Some(&member.account) != source_usage.account.as_ref()
                || member.home != source_usage.home
        })
    {
        return Err(ERR_QUOTA);
    }
    let source_bucket = selected_bucket(&source_usage, Some(&request.model)).ok_or(ERR_QUOTA)?;
    let is_exhausted = source_usage.allowed == Some(false)
        || source_bucket.blocked
        || source_bucket.windows.iter().any(|w| w.remaining == 0);
    if request.trigger == "quota" && !is_exhausted {
        return Err(ManagerError::operation(
            "codex termux: quota failure is not confirmed by current usage; work remains saved",
        ));
    }
    if request.trigger == "proactive"
        && (policy.proactive == 0
            || source_bucket
                .windows
                .iter()
                .all(|w| w.remaining > policy.proactive)
            || source_bucket.windows.is_empty())
    {
        return Err(ERR_QUOTA);
    }
    let usage = query(
        context,
        &request.profile,
        Instant::now() + Duration::from_secs(8),
    )?;
    if let Some(value) = request.flags.chunks(2).find_map(|p| {
        (p[0] == "-c")
            .then_some(p[1].strip_prefix("model_reasoning_effort="))
            .flatten()
    }) {
        let effort: String = serde_json::from_str(value).map_err(|_| ERR_USAGE)?;
        if !usage
            .efforts
            .get(&request.model)
            .is_some_and(|efforts| efforts.contains(&effort))
        {
            return Err(ManagerError::operation("codex termux: destination does not advertise the selected reasoning effort; work remains saved"));
        }
    }
    if source_usage.account == usage.account
        || reason(
            Some(&usage),
            Some(destination),
            Some(&request.model),
            policy.reserve,
            now(),
        ) != "ready"
    {
        return Err(ERR_QUOTA);
    }
    ensure_private_directory(&cache(context))?;
    let cooldown_path = cache(context).join("cooldowns.json");
    {
        let _lock = profile::directory_lock(&cache(context), libc::LOCK_EX)?;
        let mut cooldowns: BTreeMap<String, u64> = read_json(&cooldown_path)?.unwrap_or_default();
        cooldowns.retain(|_, until| *until > now());
        if cooldowns
            .get(&destination.account)
            .is_some_and(|until| *until > now())
        {
            return Err(ERR_QUOTA);
        }
        cooldowns.insert(
            source_usage.account.clone().ok_or(ERR_QUOTA)?,
            now() + policy.cooldown,
        );
        publish(&cooldown_path, &cooldowns)?;
        publish(
            &cache(context).join(format!("{}.json", request.profile)),
            &usage,
        )?;
        publish(
            &cache(context).join(format!("{source}.json")),
            &source_usage,
        )?;
    }
    let directory = cache(context).join("handoffs").join(&request.thread);
    ensure_private_directory(&directory)?;
    let _lock = profile::directory_lock(&directory, libc::LOCK_EX)?;
    let attempts_path = directory.join("attempts.json");
    let mut attempts: Vec<Attempt> = read_json(&attempts_path)?.unwrap_or_default();
    if attempts.len() > 64
        || attempts.iter().any(|a| a.turn == request.turn)
        || attempts
            .iter()
            .filter(|a| a.at > now().saturating_sub(3600))
            .count()
            >= 3
        || attempts
            .last()
            .is_some_and(|a| a.at > now().saturating_sub(policy.cooldown))
    {
        return Err(ManagerError::operation("codex termux: automatic switching paused by duplicate or cooldown guard; work remains saved"));
    }
    attempts.retain(|a| a.at > now().saturating_sub(86400));
    // Claim before exec. A crash may pause, but cannot automatically replay a continuation.
    attempts.push(Attempt {
        turn: request.turn,
        at: now(),
        source: source.to_owned(),
        target: request.profile.clone(),
    });
    publish(&attempts_path, &attempts)?;
    let continuation=(request.trigger=="quota").then_some("Continue the interrupted task from the saved conversation checkpoint. Inspect the recorded completed tool results and current workspace first. Do not replay completed commands or the original request; if any tool outcome is uncertain, ask before proceeding.");
    eprintln!(
        "Switching account {source} -> {} in the same conversation{}.",
        request.profile,
        if continuation.is_some() {
            "; continuing from the saved checkpoint"
        } else {
            " at the idle turn boundary"
        }
    );
    let target = parse_target(&OsString::from(&request.profile))?;
    let notice =
        json!({"pid":std::process::id(),"source":source,"target":request.profile}).to_string();
    profile_resume::switch_with(
        context,
        &target,
        &request.thread,
        &request.flags,
        continuation,
        Some(&notice),
    )
}
fn settings(policy: &Policy) -> Value {
    json!({"enabled":policy.enabled,"policy":policy.policy,"accounts":policy.accounts.keys().collect::<Vec<_>>(),"order":policy.order,"reserve":policy.reserve,"proactive":policy.proactive,"cooldown":policy.cooldown,"weights":policy.weights})
}
fn view(context: &Context, update: bool, model: Option<&str>) -> Result<Value, ManagerError> {
    let mut snapshot: Value =
        serde_json::from_str(&profile_view::snapshot(context)?).map_err(|_| ERR_QUOTA)?;
    let ids: Vec<String> = snapshot["profiles"]
        .as_array()
        .ok_or(ERR_QUOTA)?
        .iter()
        .filter_map(|v| v.as_str().map(str::to_owned))
        .collect();
    if update {
        refresh(context, &ids)?;
    }
    let policy = policy(context)?;
    let time = now();
    let cooldowns: BTreeMap<String, u64> =
        read_json(&cache(context).join("cooldowns.json"))?.unwrap_or_default();
    let rows:Vec<_>=ids.iter().map(|id|{
        let usage=load(context,id);
        let bucket=usage.as_ref().and_then(|u|selected_bucket(u,model));
        let windows=bucket.map(|b|b.windows.clone()).unwrap_or_default();
        let mut labels=Vec::new();
        for (minutes,label) in [(300,"5h"),(10080,"weekly")] {
            labels.push(match windows.iter().find(|w|w.minutes==minutes) {
                Some(window)=>{
                    let reset=window.reset.map(|r|if r>time{format!("reset {}m",(r-time).div_ceil(60))}else{"reset passed; refresh".into()}).unwrap_or("reset unknown".into());
                    format!("{label} {}% ({reset})",window.remaining)
                },
                None=>format!("{label} unknown"),
            });
        }
        let mut state=reason(usage.as_ref(),policy.accounts.get(id),model,policy.reserve,time);
        let excluded_until=usage.as_ref().and_then(|u|u.account.as_ref()).and_then(|a|cooldowns.get(a)).filter(|until|**until>time);
        if excluded_until.is_some(){state="cooldown";}
        json!({"id":id,"summary":format!("{} · {state}",labels.join(" · ")),"state":state,"observed":usage.as_ref().map(|u|u.observed),"windows":windows,"bucket":bucket.map(|b|&b.id),"pool":policy.accounts.contains_key(id),"excluded_until":excluded_until})
    }).collect();
    snapshot["schema"] = json!("codex-manager-profiles-v2");
    snapshot["rows"] = json!(rows);
    snapshot["rotation"] = settings(&policy);
    snapshot["selected_model"] = json!(model);
    snapshot["recommended"] = json!(recommended(
        context,
        &ids,
        &policy,
        snapshot["current"].as_str(),
        model
    ));
    snapshot["recommendation_reason"] = json!(if snapshot["recommended"].is_null() {
        "No fresh eligible enrolled account".to_owned()
    } else {
        format!(
            "Fresh eligible account ranked by {} (reserve {}%)",
            policy.policy, policy.reserve
        )
    });
    Ok(snapshot)
}
fn configure(context: &Context, value: &str) -> Result<Value, ManagerError> {
    if value.len() > 16384 {
        return Err(ERR_USAGE);
    }
    let mut update: Value = serde_json::from_str(value).map_err(|_| ERR_USAGE)?;
    let fields = update.as_object().ok_or(ERR_USAGE)?;
    if fields.keys().any(|key| {
        !matches!(
            key.as_str(),
            "enabled"
                | "policy"
                | "accounts"
                | "order"
                | "reserve"
                | "proactive"
                | "cooldown"
                | "weights"
        )
    }) {
        return Err(ERR_USAGE);
    }
    let root = manager_profiles_for_create(context)?;
    let _lock = profile::directory_lock(&root, libc::LOCK_EX)?;
    let old = policy(context)?;
    let mut merged = serde_json::to_value(&old).map_err(|_| ERR_QUOTA)?;
    if let Some(accounts) = update.get("accounts") {
        let ids = accounts
            .as_array()
            .filter(|v| v.len() <= 256)
            .ok_or(ERR_USAGE)?;
        let mut members = BTreeMap::new();
        let mut seen = BTreeSet::new();
        for id in ids {
            let id = id.as_str().ok_or(ERR_USAGE)?;
            if !seen.insert(id) {
                return Err(ERR_USAGE);
            }
            if let Some(member) = old.accounts.get(id) {
                members.insert(id.to_owned(), member.clone());
                continue;
            }
            let usage = load(context, id)
                .filter(|u| fresh(u, now()))
                .ok_or(ERR_QUOTA)?;
            let member = Member {
                home: usage.home,
                account: usage.account.ok_or(ERR_QUOTA)?,
            };
            // Editing another field never re-enrolls a replaced account implicitly.
            members.insert(id.to_owned(), member);
        }
        update["accounts"] = serde_json::to_value(members).map_err(|_| ERR_QUOTA)?;
    }
    for (key, value) in update.as_object().unwrap() {
        merged[key] = value.clone();
    }
    let mut next: Policy = serde_json::from_value(merged).map_err(|_| ERR_USAGE)?;
    next.order.retain(|id| next.accounts.contains_key(id));
    if next.enabled && next.accounts.is_empty() {
        return Err(ERR_QUOTA);
    }
    validate_policy(&next).map_err(|_| ERR_USAGE)?;
    publish(&policy_path(context), &next)?;
    Ok(settings(&next))
}
pub(super) fn run(context: &Context, args: &[OsString]) -> Result<Option<String>, ManagerError> {
    let verb = args[0].to_str().ok_or(ERR_USAGE)?;
    if verb == "rotation" {
        let value = match args.get(1).and_then(|v| v.to_str()) {
            Some("exclude") if args.len() == 4 => {
                let id = args[2].to_str().ok_or(ERR_USAGE)?;
                let seconds = args[3]
                    .to_str()
                    .and_then(|s| s.parse::<u64>().ok())
                    .filter(|s| *s <= 86400)
                    .ok_or(ERR_USAGE)?;
                let usage = load(context, id)
                    .filter(|u| fresh(u, now()))
                    .ok_or(ERR_QUOTA)?;
                let account = usage.account.ok_or(ERR_QUOTA)?;
                ensure_private_directory(&cache(context))?;
                let _lock = profile::directory_lock(&cache(context), libc::LOCK_EX)?;
                let path = cache(context).join("cooldowns.json");
                let mut values: BTreeMap<String, u64> = read_json(&path)?.unwrap_or_default();
                values.retain(|_, until| *until > now());
                if seconds == 0 {
                    values.remove(&account);
                } else {
                    values.insert(account, now() + seconds);
                }
                publish(&path, &values)?;
                json!({"profile":id,"excluded_seconds":seconds})
            }
            Some("show") if args.len() == 2 => settings(&policy(context)?),
            Some("set") if args.len() == 3 => {
                configure(context, args[2].to_str().ok_or(ERR_USAGE)?)?
            }
            Some("preview") if args.len() == 3 => view(context, false, args[2].to_str())?,
            _ => return Err(ERR_USAGE),
        };
        return Ok(Some(format!("{value}\n")));
    }
    if verb == "__quota-handoff-v1" {
        return match args {
            [_, value] => handoff(context, value.to_str().ok_or(ERR_USAGE)?),
            _ => Err(ERR_USAGE),
        };
    }
    let mut update = false;
    let mut json_output = verb == "__switch-view-v2";
    let mut model = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].to_str() {
            Some("--refresh") if !update => update = true,
            Some("--json") if !json_output => json_output = true,
            Some("--model") if model.is_none() => {
                index += 1;
                model = Some(
                    args.get(index)
                        .and_then(|v| v.to_str())
                        .filter(|s| !s.is_empty() && s.len() <= 256)
                        .ok_or(ERR_USAGE)?,
                );
            }
            _ => return Err(ERR_USAGE),
        }
        index += 1;
    }
    let value = view(context, update, model)?;
    if json_output {
        Ok(Some(format!("{value}\n")))
    } else {
        Ok(Some(
            value["rows"]
                .as_array()
                .unwrap()
                .iter()
                .map(|row| {
                    format!(
                        "{}\t{}\n",
                        row["id"].as_str().unwrap(),
                        row["summary"].as_str().unwrap()
                    )
                })
                .collect(),
        ))
    }
}

#[cfg(test)]
#[path = "quota_tests.rs"]
mod tests;
