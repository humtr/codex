use codex_chatgpt_provider::config::Config;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, PermissionsExt};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Fixture(PathBuf);
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "provider cli {} {}",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        fs::DirBuilder::new().mode(0o700).create(&path).unwrap();
        Self(path)
    }
    fn path(&self) -> PathBuf {
        self.0.join("config/codex/providers/chatgpt.toml")
    }
    fn command(&self) -> Command {
        let mut command = Command::new(env!("CARGO_BIN_EXE_codex-chatgpt-provider"));
        command
            .env_clear()
            .env("PATH", std::env::var_os("PATH").unwrap())
            .env("HOME", &self.0)
            .env("XDG_CONFIG_HOME", self.0.join("config"))
            .env("TMPDIR", &self.0);
        command
    }
    fn run(&self, args: &[&str]) -> Output {
        self.command().args(args).output().unwrap()
    }
    fn ok(&self, args: &[&str]) -> Output {
        let output = self.run(args);
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        output
    }
    fn write(&self, bytes: &[u8]) {
        fs::create_dir_all(self.path().parent().unwrap()).unwrap();
        fs::set_permissions(
            self.path().parent().unwrap(),
            fs::Permissions::from_mode(0o700),
        )
        .unwrap();
        fs::write(self.path(), bytes).unwrap();
        fs::set_permissions(self.path(), fs::Permissions::from_mode(0o600)).unwrap();
    }
    fn editor(&self, body: &str) -> PathBuf {
        let path = self.0.join("editor");
        let shell = std::env::var_os("PREFIX")
            .map(PathBuf::from)
            .map(|v| v.join("bin/sh"))
            .filter(|v| v.is_file())
            .unwrap_or_else(|| PathBuf::from("/bin/sh"));
        fs::write(&path, format!("#!{}\n{body}\n", shell.display())).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
        path
    }
    fn edit(&self, body: &str) -> Output {
        let editor = self.editor(body);
        // Editor specifications may contain spaces; quote the executable as a shell word.
        let editor = format!("'{}'", editor.display());
        self.command()
            .args(["config", "edit"])
            .env("VISUAL", editor)
            .output()
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        fs::remove_dir_all(&self.0).unwrap();
    }
}

#[test]
fn settings_read_is_readonly_and_effective_reports_defaults_without_runtime_claims() {
    let f = Fixture::new();
    for args in [
        vec!["models"],
        vec!["catalog"],
        vec!["config", "show"],
        vec!["config", "validate"],
        vec!["config", "effective"],
    ] {
        f.ok(&args);
    }
    assert!(!f.0.join("config").exists());
    let output = f.ok(&["config", "effective"]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["origins"]["context.window_tokens"], "default");
    assert_eq!(value["runtime_application"], "not_queried");
    assert_eq!(
        value["effective_model_contexts"]["chatgpt-desktop/gpt-6"]["settings"]["window_tokens"],
        1000000
    );
    let mut config = Config::default();
    config.catalog.models[0].context.window_tokens = Some(1500000);
    f.write(config.render().unwrap().as_bytes());
    let value: serde_json::Value =
        serde_json::from_slice(&f.ok(&["config", "effective"]).stdout).unwrap();
    assert_eq!(
        value["effective_model_contexts"]["chatgpt-desktop/gpt-6"]["origins"]["window_tokens"],
        "model"
    );

    let no_home = f
        .command()
        .env_remove("HOME")
        .args(["config", "show"])
        .output()
        .unwrap();
    assert!(no_home.status.success());
}

#[test]
fn public_cli_sets_dependent_values_together_and_preserves_noop_and_invalid_edits() {
    let f = Fixture::new();
    f.ok(&["config", "unset", "context.auto_compact_token_limit"]);
    assert!(!f.path().exists());
    f.ok(&[
        "config",
        "set",
        "history.mode",
        "project",
        "history.project_name",
        "Codex",
        "context.window_tokens",
        "2000000",
    ]);
    let before = fs::read(f.path()).unwrap();
    let config = Config::parse(&before).unwrap();
    assert!(config.history.create_if_missing);
    let inode = fs::metadata(f.path()).unwrap().ino();
    f.ok(&["config", "set", "history.project_name", "Codex"]);
    assert_eq!(fs::metadata(f.path()).unwrap().ino(), inode);
    assert!(!f
        .run(&[
            "config",
            "set",
            "history.project_name",
            "changed",
            "context.window_tokens",
            "0"
        ])
        .status
        .success());
    assert_eq!(fs::read(f.path()).unwrap(), before);
    f.ok(&[
        "config",
        "unset",
        "history.mode",
        "history.project_name",
        "context.window_tokens",
    ]);
    assert_eq!(
        Config::parse(&fs::read(f.path()).unwrap()).unwrap(),
        Config::default()
    );
}

#[test]
fn configured_catalog_changes_routes_and_can_add_models_without_rebuilding() {
    let f = Fixture::new();
    let mut config = Config::default();
    let mut model = config.catalog.models[0].clone();
    model.id = "chatgpt-desktop/from-user-data".into();
    model.routes[0].host_model = "user-selected-host-route".into();
    config.catalog.models.push(model);
    f.write(config.render().unwrap().as_bytes());
    f.ok(&[
        "config",
        "set",
        "model.default",
        "chatgpt-desktop/from-user-data",
    ]);
    let output = f.ok(&["models"]);
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value["source"], "file");
    assert_eq!(value["candidate_server_implemented"], true);
    assert_eq!(value["production_qualified"], false);
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("user-selected-host-route"));
}

#[test]
fn editor_rejects_failure_invalidity_and_conflicts_and_can_repair_invalid_config() {
    let f = Fixture::new();
    f.ok(&["config", "set", "context.window_tokens", "1000000"]);
    let before = fs::read(f.path()).unwrap();
    assert!(!f
        .edit("printf 'not toml = [' > \"$1\"; exit 1")
        .status
        .success());
    assert_eq!(fs::read(f.path()).unwrap(), before);
    assert!(!f.edit("printf 'not toml = [' > \"$1\"").status.success());
    assert_eq!(fs::read(f.path()).unwrap(), before);
    let conflict = f.edit("printf '[context]\\nwindow_tokens=2000000\\n' > \"$XDG_CONFIG_HOME/codex/providers/chatgpt.toml\"; printf '[context]\\nwindow_tokens=3000000\\n' > \"$1\"");
    assert!(!conflict.status.success());
    assert_eq!(
        Config::parse(&fs::read(f.path()).unwrap())
            .unwrap()
            .context
            .window_tokens,
        2_000_000
    );
    f.write(b"invalid private secret sentinel = [");
    let failed = f.run(&["config", "validate"]);
    assert!(!failed.status.success());
    assert!(!String::from_utf8_lossy(&failed.stderr).contains("secret sentinel"));
    assert!(f
        .edit("printf '[context]\\nwindow_tokens=4000000\\n' > \"$1\"")
        .status
        .success());
    assert_eq!(
        Config::parse(&fs::read(f.path()).unwrap())
            .unwrap()
            .context
            .window_tokens,
        4_000_000
    );
    assert!(!fs::read_dir(&f.0).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with("codex-chatgpt-edit-")));
}
