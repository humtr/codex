//! Foreground identity for the optional Termux Manager; title display stays upstream.
use std::fs::File;
use std::fs::OpenOptions;
use std::io;
use std::os::fd::AsRawFd;
use std::os::unix::fs::FileExt;
use std::os::unix::fs::MetadataExt;
use std::os::unix::fs::OpenOptionsExt;
use std::path::Path;
use std::process::Command;
use std::process::Stdio;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use std::time::Instant;

struct Slot {
    file: File,
    current: Option<String>,
    attempted: Option<Option<String>>,
    registered: bool,
}

impl Slot {
    fn new() -> io::Result<Self> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_err(io::Error::other)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            ".codex-terminal-thread-v1-{}-{nonce}",
            std::process::id()
        ));
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&path)?;
        std::fs::remove_file(path)?;
        Ok(Self {
            file,
            current: None,
            attempted: None,
            registered: false,
        })
    }

    fn update(&mut self, id: Option<String>) -> io::Result<()> {
        if self.current == id {
            return Ok(());
        }
        // Truncation invalidates the prior ID before publishing the new ID.
        self.file.set_len(0)?;
        self.current = None;
        if let Some(id) = &id {
            self.file.write_all_at(id.as_bytes(), 0)?;
        }
        self.current = id;
        Ok(())
    }
}

fn register(core: &Path, file: &File) -> bool {
    let Ok(metadata) = std::fs::symlink_metadata(core) else {
        return false;
    };
    if !core.is_absolute()
        || !metadata.is_file()
        || metadata.uid() != unsafe { libc::geteuid() }
        || metadata.mode() & 0o022 != 0
        || metadata.mode() & 0o111 == 0
    {
        return false;
    }
    let Ok(mut child) = Command::new(core)
        .args([
            "termux",
            "__terminal-bind-v1",
            &file.as_raw_fd().to_string(),
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    else {
        return false;
    };
    let deadline = Instant::now() + Duration::from_millis(1200);
    loop {
        match child.try_wait() {
            Ok(Some(status)) => return status.success(),
            Ok(None) if Instant::now() < deadline => thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return false;
            }
        }
    }
}

pub(crate) fn sync(thread_id: Option<String>) {
    let Some(core) = std::env::var_os("CODEX_PROFILE_CORE") else {
        return;
    };
    static SLOT: OnceLock<io::Result<Mutex<Slot>>> = OnceLock::new();
    let Ok(slot) = SLOT.get_or_init(|| Slot::new().map(Mutex::new)) else {
        return;
    };
    let Ok(mut slot) = slot.lock() else {
        return;
    };
    if slot.update(thread_id.clone()).is_err() {
        return;
    }
    if !slot.registered && slot.attempted.as_ref() != Some(&thread_id) {
        slot.attempted = Some(thread_id);
        slot.registered = register(Path::new(&core), &slot.file);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn foreground_slot_updates_without_title_or_manager() {
        let mut slot = Slot::new().unwrap();
        let path = format!("/proc/self/fd/{}", slot.file.as_raw_fd());
        assert!(
            std::fs::read_link(&path)
                .unwrap()
                .to_string_lossy()
                .ends_with(" (deleted)")
        );
        assert_eq!(slot.file.metadata().unwrap().mode() & 0o777, 0o600);
        assert_ne!(
            unsafe { libc::fcntl(slot.file.as_raw_fd(), libc::F_GETFD) } & libc::FD_CLOEXEC,
            0
        );
        for id in [
            Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3"),
            Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b4"),
            None,
        ] {
            slot.update(id.map(str::to_owned)).unwrap();
            assert_eq!(std::fs::read_to_string(&path).unwrap(), id.unwrap_or(""));
        }
    }

    #[test]
    fn native_manager_argv_and_bounded_failure() {
        if let Some(root) = std::env::var_os("CODEX_TERMINAL_TEST_ROOT") {
            let output = Path::new(&root).join("args");
            sync(Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3".into()));
            let argv = std::fs::read_to_string(&output).unwrap();
            let parts: Vec<_> = argv.lines().collect();
            assert_eq!(&parts[..2], &["termux", "__terminal-bind-v1"]);
            let descriptor: i32 = parts[2].parse().unwrap();
            let live = format!("/proc/self/fd/{descriptor}");
            assert_eq!(
                std::fs::read_to_string(&live).unwrap(),
                "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3"
            );
            sync(Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b4".into()));
            assert_eq!(
                std::fs::read_to_string(&live).unwrap(),
                "01a0fc82-dc8f-7d13-bb78-7e120f1fa9b4"
            );
            sync(None);
            assert!(std::fs::read_to_string(&live).unwrap().is_empty());
            assert_eq!(std::fs::read_to_string(&output).unwrap(), argv);
            return;
        }
        let root = std::env::temp_dir().join(format!("native-terminal-{}", std::process::id()));
        std::fs::create_dir(&root).unwrap();
        let core = root.join("core");
        let output = root.join("args");
        let shell = std::env::var("PREFIX")
            .map(|p| format!("{p}/bin/sh"))
            .unwrap_or_else(|_| "/bin/sh".into());
        let slot = Slot::new().unwrap();
        std::fs::write(
            &core,
            format!(
                "#!{shell}\nprintf '%s\\n' \"$@\" > '{}'\n",
                output.display()
            ),
        )
        .unwrap();
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700)).unwrap();
        let test_name = std::thread::current().name().unwrap().to_owned();
        for inside_tmux in [false, true] {
            let mut command = Command::new(std::env::current_exe().unwrap());
            command
                .args(["--exact", &test_name, "--nocapture"])
                .env("CODEX_TERMINAL_TEST_ROOT", &root)
                .env("CODEX_PROFILE_CORE", &core)
                .env_remove("TMUX")
                .env_remove("TMUX_PANE");
            if inside_tmux {
                command.env("TMUX", "owned-fixture").env("TMUX_PANE", "%0");
            }
            let child = command.output().unwrap();
            assert!(
                child.status.success(),
                "tmux={inside_tmux}: {}",
                String::from_utf8_lossy(&child.stdout)
            );
            assert!(
                String::from_utf8_lossy(&child.stdout).contains("1 passed; 0 failed;"),
                "nonzero child gate required"
            );
            assert!(output.exists());
            std::fs::remove_file(&output).unwrap();
        }
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o722)).unwrap();
        assert!(!register(&core, &slot.file));
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700)).unwrap();
        std::fs::write(&core, format!("#!{shell}\nexec sleep 5\n")).unwrap();
        let before = Instant::now();
        assert!(!register(&core, &slot.file));
        assert!(before.elapsed() < Duration::from_secs(3));
        std::fs::remove_dir_all(root).unwrap();
    }
}
