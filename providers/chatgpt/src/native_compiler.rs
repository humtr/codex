//! A narrow C compiler effect proof. Native supplies its resolved execution
//! environment; this module derives and versions the physical dependencies.
//! It never executes a compiler, changes argv, or supplies an alternate env.
use super::{snapshot, version, Effect, FileEffect, FileOperation};
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::path::{Path, PathBuf};

/// Produced by the qualified Native step, outside model-controlled tool args.
/// Environment values other than these paths are deliberately not transported.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Execution {
    pub schema: u32,
    pub pid: u32,
    pub start: String,
    pub thread: String,
    pub turn: String,
    pub cwd: PathBuf,
    pub shell: PathBuf,
    pub prefix: PathBuf,
    pub home: PathBuf,
    pub path: String,
    pub variables: Vec<String>,
    pub no_default_config: bool,
}

impl Execution {
    pub(crate) fn belongs_to(
        &self,
        owner: &crate::cdp::Process,
        thread: &str,
        turn: Option<&str>,
    ) -> bool {
        self.schema == 1
            && self.pid == owner.pid
            && self.start == owner.start
            && self.thread == thread
            && Some(self.turn.as_str()) == turn
    }

    fn ordinary(&self, workspace: &Path) -> bool {
        self.schema == 1
            && self.pid > 1
            && !self.start.is_empty()
            && self.start.bytes().all(|b| b.is_ascii_digit())
            && self.cwd == workspace
            && [&self.cwd, &self.shell, &self.prefix, &self.home]
                .iter()
                .all(|p| p.is_absolute() && p.as_os_str().len() <= 4096)
            && self.path.len() <= 16 * 1024
            && self.variables.len() <= 512
            && self.variables.iter().all(|key| {
                !key.is_empty()
                    && key.len() <= 256
                    && !key.bytes().any(|b| b == 0 || b == b'=')
                    && !unsafe_environment_key(key)
            })
    }
}

fn unsafe_environment_key(key: &str) -> bool {
    key.starts_with("BASH_")
        || key.starts_with("LD_")
        || key.starts_with("DYLD_")
        || key.starts_with("CCC_")
        || key.starts_with("CC_PRINT_")
        || key.starts_with("CLANG_") && key != "CLANG_NO_DEFAULT_CONFIG"
        || key.starts_with("LLVM_")
        || matches!(
            key,
            "ENV"
                | "SHELLOPTS"
                | "BASHOPTS"
                | "ZDOTDIR"
                | "GCC_EXEC_PREFIX"
                | "COMPILER_PATH"
                | "LIBRARY_PATH"
                | "CPATH"
                | "C_INCLUDE_PATH"
                | "CPLUS_INCLUDE_PATH"
                | "OBJC_INCLUDE_PATH"
                | "DEPENDENCIES_OUTPUT"
                | "SUNPRO_DEPENDENCIES"
                | "SDKROOT"
                | "MACOSX_DEPLOYMENT_TARGET"
                | "RC_DEBUG_OPTIONS"
                | "RC_DEBUG_OPTIONS_FILE"
                | "ASAN_OPTIONS"
                | "UBSAN_OPTIONS"
                | "MSAN_OPTIONS"
                | "TSAN_OPTIONS"
                | "LSAN_OPTIONS"
                | "MALLOC_TRACE"
                | "GCONV_PATH"
                | "GLIBC_TUNABLES"
        )
}

pub(super) fn metadata_version(meta: &fs::Metadata) -> String {
    format!(
        "{}:{}:{}:{}:{}:{}",
        meta.len(),
        meta.mode(),
        meta.mtime(),
        meta.mtime_nsec(),
        meta.ctime(),
        meta.ctime_nsec()
    )
}

fn metadata_input(path: &Path) -> Option<FileEffect> {
    let parent = fs::canonicalize(path.parent()?).ok()?;
    let parent_meta = fs::metadata(parent).ok()?;
    let meta = fs::metadata(path).ok()?;
    if !meta.is_file() {
        return None;
    }
    Some(FileEffect {
        path: path.into(),
        entry: format!(
            "{}:{}:{}",
            parent_meta.dev(),
            parent_meta.ino(),
            path.file_name()?.to_str()?
        ),
        inode: Some(format!("{}:{}", meta.dev(), meta.ino())),
        base: Some(metadata_version(&meta)),
        anchors: Vec::new(),
        operation: FileOperation::MetadataInput,
        linked: fs::symlink_metadata(path).ok()?.file_type().is_symlink(),
        external_input: true,
    })
}

/// Pin a negative lookup, including the first absent directory component.
/// A new config or PATH shadow cannot appear between preparation and offer.
fn absent_input(path: &Path) -> Option<FileEffect> {
    let mut missing = path.to_path_buf();
    let mut operation = FileOperation::AbsentInput;
    loop {
        match fs::symlink_metadata(&missing) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
            _ => return None,
        }
        let parent = missing.parent()?;
        match fs::canonicalize(parent) {
            Ok(real) => {
                let meta = fs::metadata(real).ok()?;
                if !meta.is_dir() {
                    return None;
                }
                return Some(FileEffect {
                    entry: format!(
                        "{}:{}:{}",
                        meta.dev(),
                        meta.ino(),
                        missing.file_name()?.to_str()?
                    ),
                    path: missing,
                    inode: None,
                    base: None,
                    anchors: Vec::new(),
                    operation,
                    linked: false,
                    external_input: true,
                });
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                missing = parent.into();
                operation = FileOperation::AbsentDirectory;
            }
            Err(_) => return None,
        }
    }
}

fn pinned_text(path: &Path, dependencies: &mut Vec<FileEffect>) -> Option<String> {
    let mut effect = snapshot(
        Path::new("/"),
        path.to_str()?,
        FileOperation::Input,
        Vec::new(),
    )?;
    effect.external_input = true;
    let bytes = fs::read(path).ok()?;
    if effect.base.as_deref() != Some(version(&bytes).as_str()) {
        return None;
    }
    dependencies.push(effect);
    String::from_utf8(bytes).ok()
}

/// Generated toolchain headers are explicit installed build inputs. An absent,
/// ambiguous or unfamiliar declaration cannot establish a compiler default.
fn macro_string(text: &str, name: &str) -> Option<Option<String>> {
    let define = format!("#define {name} ");
    let undef = format!("#undef {name}");
    let disabled = format!("/* #undef {name} */");
    let mut result = None;
    for line in text.lines().map(str::trim) {
        let value = if let Some(value) = line.strip_prefix(&define) {
            let value = value.trim().strip_prefix('"')?.strip_suffix('"')?;
            if value.contains(['\\', '"']) || value.chars().any(char::is_control) {
                return None;
            }
            Some((!value.is_empty()).then(|| value.to_owned()))
        } else if line == undef || line == disabled {
            Some(None)
        } else {
            None
        };
        if let Some(value) = value {
            if result.replace(value).is_some() {
                return None;
            }
        }
    }
    result
}

fn executable(path: &Path, prefix: &Path, dependencies: &mut Vec<FileEffect>) -> Option<PathBuf> {
    use std::io::Read;
    let real = fs::canonicalize(path).ok()?;
    let meta = fs::metadata(&real).ok()?;
    if !real.starts_with(prefix) || !meta.is_file() || meta.permissions().mode() & 0o111 == 0 {
        return None;
    }
    let mut magic = [0u8; 4];
    fs::File::open(&real).ok()?.read_exact(&mut magic).ok()?;
    if magic != *b"\x7fELF" {
        return None;
    }
    dependencies.push(metadata_input(path)?);
    Some(real)
}

fn path_tool(
    execution: &Execution,
    name: &str,
    installed: &Path,
    dependencies: &mut Vec<FileEffect>,
) -> Option<()> {
    for directory in std::env::split_paths(&execution.path) {
        // A relative PATH entry depends on shell cwd resolution and is not
        // covered by this direct installed-tool qualification.
        if !directory.is_absolute() {
            return None;
        }
        let candidate = directory.join(name);
        match fs::metadata(&candidate) {
            Ok(meta) => {
                dependencies.push(metadata_input(&candidate)?);
                if meta.is_file() && meta.permissions().mode() & 0o111 != 0 {
                    return (fs::canonicalize(candidate).ok()? == installed
                        && fs::canonicalize(directory).ok()? == installed.parent()?)
                    .then_some(());
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                dependencies.push(absent_input(&candidate)?);
            }
            Err(_) => return None,
        }
    }
    None
}

fn installed_shell(execution: &Execution, dependencies: &mut Vec<FileEffect>) -> Option<PathBuf> {
    let prefix = fs::canonicalize(&execution.prefix).ok()?;
    let shell = fs::canonicalize(&execution.shell).ok()?;
    if !["bash", "sh"].into_iter().any(|name| {
        fs::canonicalize(prefix.join("bin").join(name))
            .ok()
            .as_ref()
            == Some(&shell)
    }) {
        return None;
    }
    executable(&execution.shell, &prefix, dependencies)?;
    Some(prefix)
}

/// Shell reads use the same Native environment authority as compiler writes.
/// A command spelling alone cannot rule out BASH_ENV, exported functions,
/// dynamic loader hooks, or a different executable earlier in PATH.
pub(super) fn qualified_read(input: &str, workspace: &Path, execution: &Execution) -> Option<()> {
    if !execution.ordinary(workspace) {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    if let Some(directory) = value.get("workdir") {
        if fs::canonicalize(directory.as_str()?).ok()? != workspace {
            return None;
        }
    }
    let mut dependencies = Vec::new();
    let prefix = installed_shell(execution, &mut dependencies)?;
    let program = value["cmd"].as_str()?.split_ascii_whitespace().next()?;
    if program == "pwd" {
        return Some(());
    }
    if !matches!(program, "git" | "cat" | "ls")
        || program == "git"
            && execution.variables.iter().any(|key| {
                key.starts_with("GIT_TRACE")
                    || key.starts_with("GIT_CONFIG_")
                    || key == "GIT_EXEC_PATH"
            })
    {
        return None;
    }
    let installed = executable(
        &prefix.join("bin").join(program),
        &prefix,
        &mut dependencies,
    )?;
    path_tool(execution, program, &installed, &mut dependencies)
}

fn toolchain(execution: &Execution, link: bool, bare: bool) -> Option<(PathBuf, Vec<FileEffect>)> {
    let mut dependencies = Vec::new();
    let prefix = installed_shell(execution, &mut dependencies)?;
    let command = execution.prefix.join("bin/clang");
    let compiler = executable(&command, &prefix, &mut dependencies)?;
    // The installed driver and its executable alias must have the same
    // physical directory. Clang searches defaults and linker tools relative
    // to that directory; an alias to a driver elsewhere is a different setup.
    if compiler.parent()? != fs::canonicalize(command.parent()?).ok()? {
        return None;
    }
    if bare {
        path_tool(execution, "clang", &compiler, &mut dependencies)?;
    }
    let config = pinned_text(
        &prefix.join("include/clang/Config/config.h"),
        &mut dependencies,
    )?;
    if link {
        // This lane qualifies the installed Termux clang/lld toolchain. Other
        // link drivers and user-configured toolchains remain opaque.
        if macro_string(&config, "CLANG_DEFAULT_LINKER")?.as_deref() != Some("lld") {
            return None;
        }
        executable(&prefix.join("bin/ld.lld"), &prefix, &mut dependencies)?;
    }
    if !execution.no_default_config {
        let llvm = pinned_text(
            &prefix.join("include/llvm/Config/llvm-config.h"),
            &mut dependencies,
        )?;
        let triple = macro_string(&llvm, "LLVM_DEFAULT_TARGET_TRIPLE")??;
        // This contract is the installed Android AArch64 toolchain. Keeping
        // its OS spelling exact also excludes Clang's versioned-OS fallback
        // configuration searches (for example Darwin major/minor variants).
        if !triple
            .strip_prefix("aarch64-unknown-linux-android")
            .is_some_and(|api| !api.is_empty() && api.bytes().all(|b| b.is_ascii_digit()))
        {
            return None;
        }
        let mut directories = vec![command.parent()?.to_path_buf()];
        for (key, user) in [
            ("CLANG_CONFIG_FILE_USER_DIR", true),
            ("CLANG_CONFIG_FILE_SYSTEM_DIR", false),
        ] {
            if let Some(directory) = macro_string(&config, key)? {
                let path = if user && directory.starts_with("~/") {
                    execution.home.join(&directory[2..])
                } else if Path::new(&directory).is_absolute() {
                    PathBuf::from(directory)
                } else if !user {
                    command.parent()?.join(directory)
                } else {
                    return None;
                };
                directories.push(path);
            }
        }
        for directory in directories {
            for name in [
                format!("{triple}-clang.cfg"),
                "clang.cfg".into(),
                format!("{triple}.cfg"),
            ] {
                dependencies.push(absent_input(&directory.join(name))?);
            }
        }
    }
    Some((command, dependencies))
}

fn plain_c(bytes: &[u8]) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    !text.contains(['#', '\\'])
        && !text.contains("%:")
        && !text.contains("??")
        && !text
            .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
            .any(|token| token == "asm" || token == "_Pragma" || token.starts_with("__"))
}

pub(super) fn classify(input: &str, workspace: &Path, execution: &Execution) -> Option<Effect> {
    if !execution.ordinary(workspace) {
        return None;
    }
    let value: serde_json::Value = serde_json::from_str(input).ok()?;
    let arguments = value.as_object()?;
    if arguments.keys().any(|key| {
        !matches!(
            key.as_str(),
            "cmd" | "workdir" | "login" | "yield_time_ms" | "max_output_tokens"
        )
    }) || value["login"] != false
        || fs::canonicalize(value["workdir"].as_str()?).ok()? != workspace
    {
        return None;
    }
    let command = value["cmd"].as_str()?;
    if !command
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b' ' | b'\t' | b'/' | b'.' | b'_' | b'-'))
    {
        return None;
    }
    let words = command.split_ascii_whitespace().collect::<Vec<_>>();
    let invoked = *words.first()?;
    let args = match &words[1..] {
        ["-O0" | "-O1" | "-O2" | "-O3", remaining @ ..] => remaining,
        remaining => remaining,
    };
    let (link, source, output) = match args {
        ["-c", source, "-o", output] => (false, *source, *output),
        [source, "-o", output] => (true, *source, *output),
        _ => return None,
    };
    let operand = |value: &str| {
        !value.is_empty()
            && !value.starts_with('-')
            && value.split('/').all(|part| {
                !part.is_empty()
                    && part != "."
                    && part != ".."
                    && part
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
            })
    };
    if !operand(source) || !source.ends_with(".c") || !operand(output) {
        return None;
    }
    let (compiler, mut effects) = toolchain(execution, link, invoked == "clang")?;
    if invoked != "clang" && Some(invoked) != compiler.to_str() {
        return None;
    }
    let source_effect = snapshot(workspace, source, FileOperation::Input, Vec::new())?;
    let bytes = fs::read(workspace.join(source)).ok()?;
    if !plain_c(&bytes) || source_effect.base.as_deref() != Some(version(&bytes).as_str()) {
        return None;
    }
    let output_effect = snapshot(
        workspace,
        output,
        if workspace.join(output).exists() {
            FileOperation::Output
        } else {
            FileOperation::Add
        },
        Vec::new(),
    )?;
    if source_effect.matches(&output_effect) {
        return None;
    }
    effects.extend([source_effect, output_effect]);
    Some(Effect::Paths(effects))
}

#[cfg(test)]
pub(crate) fn fixture(workspace: &Path) -> Execution {
    let prefix = workspace.join("toolchain");
    for directory in ["bin", "include/clang/Config", "include/llvm/Config"] {
        fs::create_dir_all(prefix.join(directory)).unwrap();
    }
    // This is a filesystem/identity unit fixture, never an executable compiler
    // integration. Actual compilation is qualified on the installed Native lane.
    for name in ["clang", "ld.lld", "bash", "cat", "ls", "git"] {
        let file = prefix.join("bin").join(name);
        fs::write(&file, b"\x7fELFsynthetic-installed-tool-identity").unwrap();
        fs::set_permissions(file, fs::Permissions::from_mode(0o755)).unwrap();
    }
    fs::write(prefix.join("include/clang/Config/config.h"),
        "#define CLANG_DEFAULT_LINKER \"lld\"\n/* #undef CLANG_CONFIG_FILE_SYSTEM_DIR */\n/* #undef CLANG_CONFIG_FILE_USER_DIR */\n").unwrap();
    fs::write(
        prefix.join("include/llvm/Config/llvm-config.h"),
        "#define LLVM_DEFAULT_TARGET_TRIPLE \"aarch64-unknown-linux-android24\"\n",
    )
    .unwrap();
    let owner = crate::cdp::Process::read(std::process::id()).unwrap();
    Execution {
        schema: 1,
        pid: owner.pid,
        start: owner.start,
        thread: "compiler-test-thread".into(),
        turn: "compiler-test-turn".into(),
        cwd: workspace.into(),
        shell: prefix.join("bin/bash"),
        path: prefix.join("bin").to_str().unwrap().into(),
        prefix,
        home: workspace.into(),
        variables: ["HOME", "PATH", "PREFIX"]
            .into_iter()
            .map(str::to_owned)
            .collect(),
        no_default_config: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_effect::{can_offer, classify_with_execution, conflicts};

    fn root() -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "compiler-proof-{}",
            crate::journal::random_id().unwrap()
        ));
        fs::create_dir_all(&path).unwrap();
        fs::canonicalize(path).unwrap()
    }

    #[test]
    fn default_configuration_absence_and_path_resolution_are_versioned_inputs() {
        let root = root();
        let mut execution = fixture(&root);
        fs::write(root.join("a.c"), "int main(void){return 0;}\n").unwrap();
        let before = root.join("before");
        fs::create_dir(&before).unwrap();
        execution.path = format!("{}:{}", before.display(), execution.path);
        let input =
            serde_json::json!({"cmd":"clang -O0 a.c -o a.out","login":false,"workdir":root})
                .to_string();
        let build = classify_with_execution(
            "functions.exec_command",
            &input,
            Some(&root),
            Some(&execution),
        );
        assert!(matches!(build, Effect::Paths(_)));
        assert_eq!(can_offer(&build, &root), Ok(()));
        let original = fs::read(root.join("a.c")).unwrap();
        fs::write(root.join("replacement.c"), &original).unwrap();
        fs::rename(root.join("replacement.c"), root.join("a.c")).unwrap();
        assert_eq!(
            can_offer(&build, &root),
            Err("native_resource_path_changed"),
            "identical bytes do not transfer the claim to a different input inode"
        );
        let build = classify_with_execution(
            "functions.exec_command",
            &input,
            Some(&root),
            Some(&execution),
        );
        for target in [before.join("clang"), execution.prefix.join("bin/clang.cfg")] {
            fs::write(&target, b"changed").unwrap();
            assert_eq!(
                can_offer(&build, &root),
                Err("native_resource_version_conflict")
            );
            fs::remove_file(target).unwrap();
        }
        fs::write(execution.prefix.join("bin/clang.cfg"), "-save-temps\n").unwrap();
        assert_eq!(
            classify_with_execution(
                "functions.exec_command",
                &input,
                Some(&root),
                Some(&execution)
            ),
            Effect::Opaque
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn native_context_and_source_features_cannot_hide_additional_effects() {
        let root = root();
        let execution = fixture(&root);
        let input =
            serde_json::json!({"cmd":"clang -O0 a.c -o a.out","login":false,"workdir":root})
                .to_string();
        fs::write(root.join("a.c"), "int main(void){return 0;}\n").unwrap();
        assert_eq!(
            classify_with_execution("functions.exec_command", &input, Some(&root), None),
            Effect::Opaque
        );
        for key in [
            "BASH_ENV",
            "ENV",
            "BASH_FUNC_clang%%",
            "CCC_OVERRIDE_OPTIONS",
            "CLANG_CONFIG_FILE_USER_DIR",
            "CC_PRINT_OPTIONS_FILE",
            "COMPILER_PATH",
            "LD_PRELOAD",
        ] {
            let mut unsafe_env = execution.clone();
            unsafe_env.variables.push(key.into());
            assert_eq!(
                classify_with_execution(
                    "functions.exec_command",
                    &input,
                    Some(&root),
                    Some(&unsafe_env)
                ),
                Effect::Opaque,
                "{key}"
            );
        }
        for source in [
            "#include \"other.h\"\nint x;",
            "%:include \"other.h\"\nint x;",
            "??=include \"other.h\"\nint x;",
            "asm(\".incbin secret\");",
            "__asm__(\".incbin secret\");",
            "_Pragma(\"clang module import private\")",
            "as\\\nm(\"hidden\");",
        ] {
            fs::write(root.join("a.c"), source).unwrap();
            assert_eq!(
                classify_with_execution(
                    "functions.exec_command",
                    &input,
                    Some(&root),
                    Some(&execution)
                ),
                Effect::Opaque,
                "{source}"
            );
        }
        fs::write(root.join("a.c"), "int main(void){return 0;}\n").unwrap();
        let input =
            serde_json::json!({"cmd":"clang -O0\na.c -o a.out","login":false,"workdir":root})
                .to_string();
        assert_eq!(
            classify_with_execution(
                "functions.exec_command",
                &input,
                Some(&root),
                Some(&execution)
            ),
            Effect::Opaque
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn execution_facts_are_bound_to_the_original_native_process_and_turn() {
        let root = root();
        let execution = fixture(&root);
        let owner = crate::cdp::Process::read(std::process::id()).unwrap();
        assert!(execution.belongs_to(&owner, &execution.thread, Some(&execution.turn)));
        assert!(!execution.belongs_to(&owner, "other-thread", Some(&execution.turn)));
        assert!(!execution.belongs_to(&owner, &execution.thread, Some("other-turn")));
        assert!(!execution.belongs_to(&owner, &execution.thread, None));
        let mut stale = execution.clone();
        stale.start.push('0');
        assert!(!stale.belongs_to(&owner, &execution.thread, Some(&execution.turn)));
        stale = execution.clone();
        stale.pid += 1;
        assert!(!stale.belongs_to(&owner, &execution.thread, Some(&execution.turn)));
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn read_candidates_share_native_startup_and_path_checks() {
        let root = root();
        let execution = fixture(&root);
        for command in ["pwd", "cat a.c", "ls", "git rev-parse HEAD"] {
            let input = serde_json::json!({"cmd":command,"login":false,"workdir":root}).to_string();
            assert_eq!(
                classify_with_execution(
                    "functions.exec_command",
                    &input,
                    Some(&root),
                    Some(&execution)
                ),
                Effect::Read
            );
            assert_eq!(
                classify_with_execution("functions.exec_command", &input, Some(&root), None),
                Effect::Opaque
            );
            for key in ["BASH_ENV", "ENV", "BASH_FUNC_cat%%", "LD_PRELOAD"] {
                let mut injected = execution.clone();
                injected.variables.push(key.into());
                assert_eq!(
                    classify_with_execution(
                        "functions.exec_command",
                        &input,
                        Some(&root),
                        Some(&injected)
                    ),
                    Effect::Opaque
                );
            }
        }
        let shadow = root.join("shadow");
        fs::create_dir(&shadow).unwrap();
        let cat = shadow.join("cat");
        fs::write(&cat, b"\x7fELFunqualified").unwrap();
        fs::set_permissions(&cat, fs::Permissions::from_mode(0o755)).unwrap();
        let mut injected = execution.clone();
        injected.path = format!("{}:{}", shadow.display(), execution.path);
        let input = serde_json::json!({"cmd":"cat a.c","login":false}).to_string();
        assert_eq!(
            classify_with_execution(
                "functions.exec_command",
                &input,
                Some(&root),
                Some(&injected)
            ),
            Effect::Opaque
        );
        injected = execution.clone();
        injected.variables.push("GIT_TRACE".into());
        let input = serde_json::json!({"cmd":"git rev-parse HEAD","login":false}).to_string();
        assert_eq!(
            classify_with_execution(
                "functions.exec_command",
                &input,
                Some(&root),
                Some(&injected)
            ),
            Effect::Opaque
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn existing_binary_outputs_remain_versioned_exclusive_writes() {
        let root = root();
        let execution = fixture(&root);
        fs::write(root.join("a.c"), "int main(void){return 0;}\n").unwrap();
        fs::write(root.join("a.out"), b"\x7fELF\xff\x80binary").unwrap();
        let input = serde_json::json!({"cmd":"clang a.c -o a.out","login":false,"workdir":root})
            .to_string();
        let build = classify_with_execution(
            "functions.exec_command",
            &input,
            Some(&root),
            Some(&execution),
        );
        assert!(matches!(build, Effect::Paths(_)));
        assert_eq!(can_offer(&build, &root), Ok(()));
        assert!(conflicts(&build, &build));
        let replacement = root.join("replacement.out");
        fs::write(&replacement, fs::read(root.join("a.out")).unwrap()).unwrap();
        fs::rename(replacement, root.join("a.out")).unwrap();
        assert_eq!(
            can_offer(&build, &root),
            Err("native_resource_path_changed")
        );
        let build = classify_with_execution(
            "functions.exec_command",
            &input,
            Some(&root),
            Some(&execution),
        );
        fs::write(root.join("a.out"), b"\x7fELFchanged").unwrap();
        assert_eq!(
            can_offer(&build, &root),
            Err("native_resource_version_conflict")
        );
        fs::remove_dir_all(root).unwrap();
    }
}
