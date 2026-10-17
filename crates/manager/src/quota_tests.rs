use super::*;

fn sample() -> Usage {
    Usage {
        schema: 1,
        home: [1, 2],
        auth: Vec::new(),
        observed: 1000,
        status: "ready".into(),
        account: Some("account-a".into()),
        allowed: Some(true),
        models: vec!["owned".into()],
        buckets: vec![Bucket {
            id: "codex".into(),
            model: None,
            blocked: false,
            windows: vec![
                Window {
                    minutes: 300,
                    remaining: 80,
                    reset: Some(1500),
                },
                Window {
                    minutes: 10080,
                    remaining: 50,
                    reset: Some(9000),
                },
            ],
        }],
    }
}
#[test]
fn quota_unknown_stale_changed_permission_model_and_either_window_refuse() {
    let u = sample();
    let m = Member {
        home: u.home,
        account: u.account.clone().unwrap(),
    };
    assert_eq!(reason(Some(&u), Some(&m), Some("owned"), 5, 1000), "ready");
    assert_ne!(reason(Some(&u), Some(&m), Some("owned"), 5, 1091), "ready");
    assert_ne!(reason(Some(&u), Some(&m), Some("owned"), 5, 999), "ready");
    assert_ne!(reason(Some(&u), Some(&m), Some("other"), 5, 1000), "ready");
    for index in 0..2 {
        let mut bad = u.clone();
        bad.buckets[0].windows[index].remaining = 0;
        bad.buckets[0].windows[index].reset = Some(999);
        assert_ne!(
            reason(Some(&bad), Some(&m), Some("owned"), 5, 1000),
            "ready"
        );
    }
    for allowed in [None, Some(false)] {
        let mut bad = u.clone();
        bad.allowed = allowed;
        assert_ne!(reason(Some(&bad), Some(&m), None, 5, 1000), "ready");
    }
    let other = Member {
        home: [1, 3],
        account: m.account.clone(),
    };
    assert_ne!(reason(Some(&u), Some(&other), None, 5, 1000), "ready");
    let other = Member {
        home: m.home,
        account: "replacement".into(),
    };
    assert_ne!(reason(Some(&u), Some(&other), None, 5, 1000), "ready");
}
#[test]
fn quota_labels_use_duration_and_missing_or_duplicate_windows_do_not_invent_capacity() {
    assert!(
        !bucket(
            &json!({"rateLimitReachedType":"rate_limit_reached"}),
            "codex"
        )
        .unwrap()
        .blocked
    );
    let b=bucket(&json!({"primary":{"usedPercent":15,"windowDurationMins":10080,"resetsAt":1200},"secondary":{"usedPercent":40,"windowDurationMins":300}}),"codex").unwrap();
    assert_eq!(b.windows[0].minutes, 10080);
    assert_eq!(b.windows[1].remaining, 60);
    assert!(bucket(
        &json!({"primary":{"usedPercent":101,"windowDurationMins":300}}),
        "codex"
    )
    .is_err());
    assert!(bucket(&json!({"primary":{"usedPercent":10,"windowDurationMins":300},"secondary":{"usedPercent":20,"windowDurationMins":300}}),"codex").is_err());
    let mut u = sample();
    u.buckets[0].windows.clear();
    assert_ne!(reason(Some(&u), None, None, 5, 1000), "ready");
}

#[test]
fn quota_native_read_client_only_uses_account_and_model_methods_and_kills_timed_out_child() {
    let mut f = Fixture::new();
    let core = f.root.join("core");
    let program = f.root.join("core.py");
    let journal = f.root.join("methods");
    fs::write(&core, "#!/bin/sh\nexec python3 \"$0.py\" \"$@\"\n").unwrap();
    set_mode(&core, 0o700).unwrap();
    fs::write(&program,format!(r#"import sys,json,os,time
assert sys.argv[1:]==['app-server']
for line in sys.stdin:
 v=json.loads(line);method=v['method']
 with open({journal:?},'a') as f:f.write(method+'\n')
 if 'id' not in v:continue
 if method=='initialize':result={{}}
 elif method=='account/read':result={{'account':{{'type':'chatgpt'}},'workspaceRouting':{{'chatgptAccountId':'a'}}}}
 elif method=='account/rateLimits/read':result={{'accountId':'a','ordinaryUsageAllowed':True,'rateLimits':{{'limitId':'codex','primary':{{'usedPercent':20,'windowDurationMins':300,'resetsAt':9999999999}}}}}}
 elif method=='model/list':result={{'data':[{{'model':'owned'}}],'nextCursor':None}}
 else:raise Exception('unexpected method')
 print(json.dumps({{'id':v['id'],'result':result}}),flush=True)
"#,journal=journal.to_str().unwrap())).unwrap();
    f.context.core_entrypoint = core.clone();
    let result = query(
        &f.context,
        "default",
        Instant::now() + Duration::from_secs(3),
    )
    .unwrap();
    assert_eq!(result.account.as_deref(), Some("a"));
    assert_eq!(result.models, ["owned"]);
    assert_eq!(
        fs::read_to_string(&journal).unwrap(),
        "initialize\ninitialized\naccount/read\naccount/rateLimits/read\nmodel/list\n"
    );
    fs::write(&program, "import time\ntime.sleep(5)\n").unwrap();
    let started = Instant::now();
    assert!(query(
        &f.context,
        "default",
        Instant::now() + Duration::from_millis(50)
    )
    .is_err());
    assert!(started.elapsed() < Duration::from_secs(2));
}
#[test]
fn quota_policy_orders_both_windows_without_summing_percentages() {
    let mut a = sample();
    let mut b = sample();
    b.buckets[0].windows[0].reset = Some(1100);
    b.buckets[0].windows[1].remaining = 20;
    let mut p = Policy {
        policy: "5h".into(),
        ..Default::default()
    };
    assert!(score(&b, &p, None, 1000) < score(&a, &p, None, 1000));
    p.policy = "remaining".into();
    assert!(score(&a, &p, None, 1000) < score(&b, &p, None, 1000));
    p.policy = "weekly".into();
    a.buckets[0].windows[1].reset = Some(1050);
    assert!(score(&a, &p, None, 1000) < score(&b, &p, None, 1000));
    p.policy = "custom".into();
    p.weights = [0, 10, 0];
    assert!(score(&a, &p, None, 1000) < score(&b, &p, None, 1000));
    p.reserve = 100;
    assert!(validate_policy(&p).is_err());
    p.reserve = 5;
    p.policy = "sh -c command".into();
    assert!(validate_policy(&p).is_err());
}

struct Fixture {
    root: PathBuf,
    context: Context,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "codex-quota-{}-{}",
            std::process::id(),
            TEMP_COUNTER.fetch_add(1, Ordering::Relaxed)
        ));
        fs::create_dir(&root).unwrap();
        set_mode(&root, 0o700).unwrap();
        let context = Context {
            home: root.clone(),
            inherited_codex_home: None,
            core_entrypoint: std::env::current_exe().unwrap(),
        };
        fs::create_dir(root.join(".codex")).unwrap();
        set_mode(&root.join(".codex"), 0o700).unwrap();
        Self { root, context }
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.root).unwrap();
    }
}
fn seed(f: &Fixture, id: &str, account: &str, remaining: i64) {
    if id != "default" {
        create_profile(&f.context, id).unwrap();
    }
    let mut usage = sample();
    let home = home_for(&f.context, id).unwrap();
    usage.home = identity(&home).unwrap();
    usage.observed = now();
    usage.account = Some(account.into());
    usage.buckets[0].windows[1].remaining = remaining;
    ensure_private_directory(&cache(&f.context)).unwrap();
    publish(&cache(&f.context).join(format!("{id}.json")), &usage).unwrap();
}
#[test]
fn quota_duplicate_accounts_binding_replacement_and_invalid_settings_preserve_policy() {
    let f = Fixture::new();
    seed(&f, "default", "a", 50);
    seed(&f, "same", "a", 50);
    seed(&f, "other", "b", 30);
    configure(
        &f.context,
        r#"{"accounts":["default","same","other"],"enabled":true}"#,
    )
    .unwrap();
    let ids = vec!["default".into(), "same".into(), "other".into()];
    let p = policy(&f.context).unwrap();
    assert_eq!(
        recommended(&f.context, &ids, &p, Some("default"), Some("owned")),
        Some("other".into())
    );
    let before = fs::read(policy_path(&f.context)).unwrap();
    assert!(configure(&f.context, r#"{"reserve":100}"#).is_err());
    assert_eq!(fs::read(policy_path(&f.context)).unwrap(), before);
    let mut replacement = load(&f.context, "other").unwrap();
    replacement.account = Some("c".into());
    publish(&cache(&f.context).join("other.json"), &replacement).unwrap();
    assert_eq!(
        recommended(&f.context, &ids, &p, Some("default"), Some("owned")),
        None
    );
    configure(&f.context, r#"{"policy":"weekly"}"#).unwrap();
    assert_eq!(policy(&f.context).unwrap().accounts, p.accounts);
    let view = view(&f.context, false, Some("owned")).unwrap();
    assert!(!view.to_string().contains("account-a"));
    assert_eq!(view["schema"], "codex-manager-profiles-v2");
    let old: Value = serde_json::from_str(&profile_view::snapshot(&f.context).unwrap()).unwrap();
    assert_eq!(old["schema"], "codex-manager-profiles-v1");
    assert!(old.get("rows").is_none());
}
#[test]
fn quota_cache_refuses_symlinks_nonprivate_future_and_replaced_homes() {
    let f = Fixture::new();
    seed(&f, "default", "a", 50);
    let path = cache(&f.context).join("default.json");
    set_mode(&path, 0o644).unwrap();
    assert!(load(&f.context, "default").is_none());
    set_mode(&path, 0o600).unwrap();
    let mut usage = load(&f.context, "default").unwrap();
    usage.observed = now() + 100;
    publish(&path, &usage).unwrap();
    assert!(!fresh(&load(&f.context, "default").unwrap(), now()));
    fs::rename(f.root.join(".codex"), f.root.join("retired")).unwrap();
    fs::create_dir(f.root.join(".codex")).unwrap();
    set_mode(&f.root.join(".codex"), 0o700).unwrap();
    assert!(load(&f.context, "default").is_none());
}
