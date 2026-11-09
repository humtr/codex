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
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::thread;
use std::time::Duration;
use std::time::Instant;

struct Slot {
    file: Arc<File>,
    current: Option<String>,
    attempted: Option<Option<String>>,
    registered: bool,
    registration: Option<thread::JoinHandle<bool>>,
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
            file: Arc::new(file),
            current: None,
            attempted: None,
            registered: false,
            registration: None,
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

    fn sync(&mut self, core: &Path, thread_id: Option<String>) -> io::Result<()> {
        self.update(thread_id.clone())?;
        if self
            .registration
            .as_ref()
            .is_some_and(thread::JoinHandle::is_finished)
        {
            self.registered = self
                .registration
                .take()
                .is_some_and(|task| task.join().unwrap_or(false));
        }
        if self.registered
            || self.registration.is_some()
            || self.attempted.as_ref() == Some(&thread_id)
        {
            return Ok(());
        }
        self.attempted = Some(thread_id);
        // Registration owns no foreground state. Keep the same descriptor alive
        // while the optional Manager command waits, without locking out rendering
        // or updates to the currently displayed thread.
        let file = Arc::clone(&self.file);
        let core = core.to_owned();
        self.registration = Some(
            thread::Builder::new()
                .name("terminal-registration".into())
                .spawn(move || register(&core, &file))?,
        );
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
    let _ = slot.sync(Path::new(&core), thread_id);
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
    fn pending_registration_keeps_foreground_updates_live_on_the_same_descriptor() {
        let root = tempfile::tempdir().unwrap();
        let core = root.path().join("core");
        let started = root.path().join("started");
        let release = root.path().join("release");
        let shell = std::env::var("PREFIX")
            .map(|p| format!("{p}/bin/sh"))
            .unwrap_or_else(|_| "/bin/sh".into());
        std::fs::write(
            &core,
            format!(
                "#!{shell}\nprintf '%s\\n' \"$@\" >> '{}'\nwhile [ ! -f '{}' ]; do sleep 0.01; done\n",
                started.display(),
                release.display(),
            ),
        )
        .unwrap();
        std::fs::set_permissions(&core, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut slot = Slot::new().unwrap();
        let descriptor = slot.file.as_raw_fd();
        let path = format!("/proc/self/fd/{descriptor}");
        slot.sync(&core, Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3".into()))
            .unwrap();
        let deadline = Instant::now() + Duration::from_secs(1);
        while !started.exists() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        assert!(
            started.exists(),
            "registration must actually enter the helper"
        );
        assert!(!slot.registration.as_ref().unwrap().is_finished());
        for id in [Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b4"), None] {
            slot.sync(&core, id.map(str::to_owned)).unwrap();
            assert_eq!(slot.file.as_raw_fd(), descriptor);
            assert_eq!(std::fs::read_to_string(&path).unwrap(), id.unwrap_or(""));
            assert!(!slot.registration.as_ref().unwrap().is_finished());
        }
        std::fs::write(&release, b"ready").unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while !slot.registration.as_ref().unwrap().is_finished() && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(5));
        }
        slot.sync(&core, None).unwrap();
        assert!(slot.registered);
        assert!(slot.registration.is_none());
        assert_eq!(
            std::fs::read_to_string(&started).unwrap(),
            format!("termux\n__terminal-bind-v1\n{descriptor}\n"),
            "foreground changes must not spawn overlapping registration commands",
        );
        assert!(std::fs::read_to_string(&path).unwrap().is_empty());
    }

    #[test]
    fn native_manager_argv_and_bounded_failure() {
        if let Some(root) = std::env::var_os("CODEX_TERMINAL_TEST_ROOT") {
            let output = Path::new(&root).join("args");
            sync(Some("01a0fc82-dc8f-7d13-bb78-7e120f1fa9b3".into()));
            let deadline = Instant::now() + Duration::from_secs(2);
            while !output.exists() && Instant::now() < deadline {
                thread::sleep(Duration::from_millis(5));
            }
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
        // The Manager supports direct terminal registration only.
        let child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", &test_name, "--nocapture"])
            .env("CODEX_TERMINAL_TEST_ROOT", &root)
            .env("CODEX_PROFILE_CORE", &core)
            .env_remove("TMUX")
            .env_remove("TMUX_PANE")
            .output()
            .unwrap();
        assert!(
            child.status.success(),
            "{}",
            String::from_utf8_lossy(&child.stdout)
        );
        assert!(
            String::from_utf8_lossy(&child.stdout).contains("1 passed; 0 failed;"),
            "nonzero child gate required"
        );
        assert!(output.exists());
        std::fs::remove_file(&output).unwrap();
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
