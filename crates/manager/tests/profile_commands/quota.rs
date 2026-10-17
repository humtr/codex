use super::*;
use serde_json::{json, Value};
use std::os::fd::AsRawFd;

struct Fixture {
    root: TestRoot,
    core: PathBuf,
    target: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let root = TestRoot::new();
        let core = root.0.join("core");
        fs::create_dir(root.0.join(".codex")).unwrap();
        fs::set_permissions(root.0.join(".codex"), fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(&core, "#!/bin/sh\nexec python3 \"$0.py\" \"$@\"\n").unwrap();
        fs::set_permissions(&core, fs::Permissions::from_mode(0o700)).unwrap();
        fs::write(root.0.join("core.py"),r#"import os,sys,json
home=os.environ['CODEX_HOME']
if sys.argv[1]=='resume':
 with open(os.path.join(os.environ['HOME'],'resumes'),'a') as f:f.write(json.dumps({'args':sys.argv[1:],'home':home})+'\n')
 sys.exit(0)
assert sys.argv[1:]==['app-server']
facts=json.load(open(os.path.join(home,'fixture.json')))
for line in sys.stdin:
 v=json.loads(line)
 if 'id' not in v:continue
 method=v['method']
 if method=='initialize':r={}
 elif method=='account/read':r={'account':{'type':'chatgpt'},'workspaceRouting':{'chatgptAccountId':facts['account']}}
 elif method=='account/rateLimits/read':r={'accountId':facts['account'],'ordinaryUsageAllowed':facts['allowed'],'rateLimits':{'limitId':'codex','primary':{'usedPercent':100-facts['remaining'],'windowDurationMins':300,'resetsAt':9999999999},'secondary':{'usedPercent':10,'windowDurationMins':10080,'resetsAt':9999999999}}}
 elif method=='model/list':r={'data':[{'model':'owned','supportedReasoningEfforts':[{'reasoningEffort':'high'}]}],'nextCursor':None}
 else:raise Exception('unexpected read method')
 print(json.dumps({'id':v['id'],'result':r}),flush=True)
"#).unwrap();
        let target = root.0.join(".local/share/codex/manager/profiles/next/home");
        let f = Self { root, core, target };
        f.run(&["profile", "create", "next"], true);
        f.facts(false, "b", 80);
        f.run(&["usage", "--refresh", "--json", "--model", "owned"], true);
        f.run(
            &[
                "rotation",
                "set",
                r#"{"accounts":["default","next"],"enabled":true,"cooldown":30}"#,
            ],
            true,
        );
        f
    }
    fn facts(&self, source_allowed: bool, target_account: &str, target_remaining: i64) {
        fs::write(self.root.0.join(".codex/fixture.json"),json!({"account":"a","allowed":source_allowed,"remaining":if source_allowed{10}else{0}}).to_string()).unwrap();
        fs::write(
            self.target.join("fixture.json"),
            json!({"account":target_account,"allowed":true,"remaining":target_remaining})
                .to_string(),
        )
        .unwrap();
    }
    fn run(&self, args: &[&str], success: bool) -> Output {
        let out = base_manager_command(&self.root.0, &self.core)
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("CODEX_HOME", self.root.0.join(".codex"))
            .args(args)
            .output()
            .unwrap();
        assert_eq!(
            out.status.success(),
            success,
            "{args:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    }
    fn handoff(&self, trigger: &str, turn: &str, success: bool) -> Output {
        let flags = [
            "--model",
            "owned",
            "-c",
            "approval_policy=\"on-request\"",
            "-c",
            "approvals_reviewer=\"auto_review\"",
            "-c",
            "sandbox_mode=\"danger-full-access\"",
            "-c",
            "model_reasoning_effort=\"high\"",
        ];
        let request = json!({"profile":"next","thread":"12345678-1234-1234-1234-123456789abc","turn":turn,"model":"owned","trigger":trigger,"flags":flags});
        self.run(&["__quota-handoff-v1", &request.to_string()], success)
    }
    fn resumes(&self) -> Vec<Value> {
        fs::read_to_string(self.root.0.join("resumes"))
            .unwrap_or_default()
            .lines()
            .map(|line| serde_json::from_str(line).unwrap())
            .collect()
    }
}

#[test]
fn quota_handoff_claim_preserves_uuid_flags_destination_and_never_replays_original_request() {
    let f = Fixture::new();
    f.handoff("quota", "failed-turn", true);
    let records = f.resumes();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0]["home"], f.target.to_str().unwrap());
    let args = records[0]["args"].as_array().unwrap();
    assert_eq!(args[0], "resume");
    assert_eq!(args[1], "12345678-1234-1234-1234-123456789abc");
    assert!(args
        .iter()
        .any(|x| x == "approvals_reviewer=\"auto_review\""));
    assert!(args.iter().any(|x| x == "model_reasoning_effort=\"high\""));
    assert!(args
        .last()
        .unwrap()
        .as_str()
        .unwrap()
        .contains("saved conversation checkpoint"));
    f.handoff("quota", "failed-turn", false);
    f.handoff("quota", "another-turn", false);
    assert_eq!(f.resumes().len(), 1);
    assert_eq!(
        serde_json::from_slice::<Value>(&fs::read(f.root.0.join(".codex/fixture.json")).unwrap())
            .unwrap()["account"],
        "a"
    );
}
#[test]
fn quota_handoff_revalidates_changed_identity_exhaustion_and_destination_capacity() {
    for case in ["identity", "capacity", "unconfirmed"] {
        let f = Fixture::new();
        match case {
            "identity" => f.facts(false, "replacement", 80),
            "capacity" => f.facts(false, "b", 0),
            _ => f.facts(true, "b", 80),
        };
        f.handoff("quota", "failed-turn", false);
        assert!(f.resumes().is_empty(), "{case}");
    }
}
#[test]
fn quota_proactive_handoff_is_opt_in_and_adds_no_continuation_prompt() {
    let f = Fixture::new();
    f.facts(true, "b", 80);
    f.handoff("proactive", "complete-turn", false);
    assert!(f.resumes().is_empty());
    f.run(&["rotation", "set", r#"{"proactive":15}"#], true);
    f.handoff("proactive", "complete-turn", true);
    assert!(!f.resumes()[0]["args"]
        .as_array()
        .unwrap()
        .last()
        .unwrap()
        .as_str()
        .unwrap()
        .contains("checkpoint"));
}
#[test]
fn quota_handoff_never_removes_or_steals_an_existing_writer() {
    let f = Fixture::new();
    let directory = f.root.0.join(".codex/thread-writer-locks");
    fs::create_dir(&directory).unwrap();
    fs::set_permissions(&directory, fs::Permissions::from_mode(0o700)).unwrap();
    let coordination = directory.join(".coordination.lock");
    let lock = directory.join("12345678-1234-1234-1234-123456789abc.lock");
    for path in [&coordination, &lock] {
        fs::write(path, "").unwrap();
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).unwrap();
    }
    let owner = fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open(&lock)
        .unwrap();
    assert_eq!(
        unsafe { libc::flock(owner.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) },
        0
    );
    let before = fs::metadata(&lock).unwrap();
    f.handoff("quota", "failed-turn", false);
    assert!(f.resumes().is_empty());
    use std::os::unix::fs::MetadataExt;
    assert_eq!(fs::metadata(&lock).unwrap().ino(), before.ino());
}
