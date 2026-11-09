//! Account-local browser handoff for the Provider-owned Linux ChatGPT app.
//! The official app writes an external-login URL to its private bind mount.
//! This owner alone validates and opens approved OAuth links on Android.
use crate::{journal::random_id, protocol::Error};
use axum::http::Uri;
use percent_encoding::percent_decode_str;
use std::{
    fs::{self, File},
    io::{Read, Write},
    os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt},
    path::Path,
    time::Duration,
};
use tokio::{task::JoinHandle, time::interval};
use tokio_util::sync::CancellationToken;

const DESKTOP_ENTRY: &str = "[Desktop Entry]\nType=Application\nName=Termux authentication browser\nExec=/ui/bin/xdg-open %u\nNoDisplay=true\nTerminal=false\nMimeType=x-scheme-handler/http;x-scheme-handler/https;\n";
const OPENER: &str = "#!/bin/sh\nset -eu\ntest \"$#\" = 1\numask 077\nprintf '%s' \"$1\" > /ui/open-url.tmp\nmv /ui/open-url.tmp /ui/open-url\n";
const DEFAULT: &str = "codex-termux-auth-browser.desktop";
const MAX_URL: u64 = 16_383;

fn owned_directory(path: &Path) -> Result<(), Error> {
    if !path.exists() {
        fs::DirBuilder::new()
            .mode(0o700)
            .create(path)
            .map_err(|_| Error("host_browser_directory_unavailable"))?;
    }
    let m = fs::symlink_metadata(path).map_err(|_| Error("host_browser_directory_unavailable"))?;
    if !m.is_dir() || m.uid() != unsafe { libc::geteuid() } || m.mode() & 0o022 != 0 {
        return Err(Error("host_browser_directory_unavailable"));
    }
    Ok(())
}

fn write_private(path: &Path, bytes: &[u8], executable: bool) -> Result<(), Error> {
    let parent = path
        .parent()
        .ok_or(Error("host_browser_directory_unavailable"))?;
    owned_directory(parent)?;
    let name = format!(".auth-{}", &random_id()?[..16]);
    let temporary = parent.join(name);
    let mut file = File::options()
        .write(true)
        .create_new(true)
        .mode(if executable { 0o700 } else { 0o600 })
        .open(&temporary)
        .map_err(|_| Error("host_browser_configuration_unavailable"))?;
    let result = file
        .write_all(bytes)
        .and_then(|_| file.sync_all())
        .and_then(|_| fs::rename(&temporary, path));
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(Error("host_browser_configuration_unavailable"));
    }
    Ok(())
}

fn browser_defaults(previous: &str) -> String {
    let mut output = String::new();
    let mut in_defaults = false;
    let mut seen = false;
    let entries = format!("x-scheme-handler/http={DEFAULT}\nx-scheme-handler/https={DEFAULT}\n");
    for line in previous.lines() {
        if line.starts_with('[') {
            if in_defaults {
                output.push_str(&entries);
            }
            in_defaults = line == "[Default Applications]";
            seen |= in_defaults;
        } else if in_defaults
            && (line.starts_with("x-scheme-handler/http=")
                || line.starts_with("x-scheme-handler/https="))
        {
            continue;
        }
        output.push_str(line);
        output.push('\n');
    }
    if in_defaults {
        output.push_str(&entries);
    }
    if !seen {
        output.push_str("[Default Applications]\n");
        output.push_str(&entries);
    }
    output
}

fn prepare(home: &Path, temporary: &Path) -> Result<(), Error> {
    let bin = temporary.join("bin");
    owned_directory(&bin)?;
    write_private(&bin.join("xdg-open"), OPENER.as_bytes(), true)?;
    let local = home.join(".local");
    owned_directory(&local)?;
    let share = local.join("share");
    owned_directory(&share)?;
    let applications = share.join("applications");
    owned_directory(&applications)?;
    write_private(&applications.join(DEFAULT), DESKTOP_ENTRY.as_bytes(), false)?;
    let config = home.join(".config");
    owned_directory(&config)?;
    let mime = config.join("mimeapps.list");
    let old = match fs::symlink_metadata(&mime) {
        Ok(m) if m.is_file() && m.uid() == unsafe { libc::geteuid() } => fs::read_to_string(&mime)
            .map_err(|_| Error("host_browser_configuration_unavailable"))?,
        Ok(_) => return Err(Error("host_browser_configuration_unavailable")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(_) => return Err(Error("host_browser_configuration_unavailable")),
    };
    let config = browser_defaults(&old);
    if config != old {
        write_private(&mime, config.as_bytes(), false)?;
    }
    Ok(())
}

/// Match the established mobile desktop's OAuth allow-list. Never log or
/// persist the callback, state, challenge, or URL outside the owner temp root.
fn permitted_url(url: &str, nested: bool) -> bool {
    if url.len() > MAX_URL as usize
        || url.chars().any(|c| c.is_whitespace() || c.is_control())
        || url.contains('#')
    {
        return false;
    }
    let Ok(uri) = url.parse::<Uri>() else {
        return false;
    };
    if uri.scheme_str() != Some("https")
        || uri.authority().is_none_or(|a| a.as_str().contains('@'))
        || !matches!(uri.port_u16(), None | Some(443))
    {
        return false;
    }
    let query = uri.query().unwrap_or("");
    let mut nested_target = None;
    for entry in query.split('&') {
        let (key, value) = entry.split_once('=').unwrap_or((entry, ""));
        let Ok(key) = percent_decode_str(key).decode_utf8() else {
            return false;
        };
        if matches!(key.as_ref(), "id_token" | "access_token" | "refresh_token") {
            return false;
        }
        if key == "authorize_url" {
            if nested_target.is_some() {
                return false;
            }
            let Ok(target) = percent_decode_str(value).decode_utf8() else {
                return false;
            };
            nested_target = Some(target.into_owned());
        }
    }
    match (uri.host(), uri.path()) {
        (Some("auth.openai.com"), "/oauth/authorize") => true,
        (Some("auth.openai.com"), path) if path.starts_with("/api/accounts/") => true,
        (Some("chatgpt.com"), "/codex/desktop-auth") if !nested => {
            nested_target.is_some_and(|target| permitted_url(&target, true))
        }
        _ => false,
    }
}

fn take_request(path: &Path) -> Option<String> {
    let result = (|| {
        let m = fs::symlink_metadata(path).ok()?;
        if !m.is_file() || m.uid() != unsafe { libc::geteuid() } || m.len() > MAX_URL {
            return None;
        }
        let file = File::options()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW)
            .open(path)
            .ok()?;
        let mut bytes = Vec::new();
        file.take(MAX_URL + 1).read_to_end(&mut bytes).ok()?;
        String::from_utf8(bytes).ok()
    })();
    // The validated host owns this temporary directory and consumes each URL
    // once; no request survives a failed launch or a host restart.
    let _ = fs::remove_file(path);
    result.filter(|url| permitted_url(url, false))
}

pub(crate) struct AuthBrowser {
    stop: CancellationToken,
    worker: JoinHandle<()>,
}

impl AuthBrowser {
    pub(crate) fn start(home: &Path, temporary: &Path, prefix: &Path) -> Result<Self, Error> {
        prepare(home, temporary)?;
        let url_path = temporary.join("open-url");
        let activity = prefix.join("bin/termux-am-socket");
        let stop = CancellationToken::new();
        let shutdown = stop.clone();
        let worker = tokio::spawn(async move {
            let mut tick = interval(Duration::from_millis(250));
            loop {
                tokio::select! {
                    _ = shutdown.cancelled() => break,
                    _ = tick.tick() => {
                        if let Some(url) = take_request(&url_path) {
                            let _ = crate::runtime::request_viewer_activity(&activity, &url).await;
                        }
                    }
                }
            }
        });
        Ok(Self { stop, worker })
    }

    pub(crate) fn stop(&self) {
        self.stop.cancel();
        self.worker.abort();
    }
}

impl Drop for AuthBrowser {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_supported_auth_redirects_are_forwarded() {
        assert!(permitted_url(
            "https://auth.openai.com/oauth/authorize?state=test",
            false
        ));
        assert!(permitted_url(
            "https://auth.openai.com/api/accounts/abc",
            false
        ));
        assert!(permitted_url("https://chatgpt.com/codex/desktop-auth?authorize_url=https%3A%2F%2Fauth.openai.com%2Foauth%2Fauthorize%3Fstate%3Dtest", false));
        for unsafe_url in [
            "https://evil.example/oauth/authorize",
            "http://auth.openai.com/oauth/authorize",
            "https://auth.openai.com.evil.example/oauth/authorize",
            "https://auth.openai.com/oauth/authorize?access_token=secret",
            "https://chatgpt.com/codex/desktop-auth?authorize_url=https%3A%2F%2Fevil.example%2F",
            "https://auth.openai.com/oauth/authorize#token",
            "https://user@auth.openai.com/oauth/authorize",
        ] {
            assert!(
                !permitted_url(unsafe_url, false),
                "unsafe OAuth target admitted"
            );
        }
    }

    #[test]
    fn mime_defaults_preserve_other_app_associations() {
        let old = "[Default Applications]\ntext/plain=other.desktop\nx-scheme-handler/http=old.desktop\n[Added Associations]\ntext/plain=other.desktop;\n";
        let result = browser_defaults(old);
        assert!(result.contains("text/plain=other.desktop"));
        assert!(result.contains("x-scheme-handler/http=codex-termux-auth-browser.desktop"));
        assert!(result.contains("x-scheme-handler/https=codex-termux-auth-browser.desktop"));
        assert!(!result.contains("old.desktop"));
        assert!(result.contains("[Added Associations]\ntext/plain=other.desktop;"));
    }

    #[tokio::test]
    async fn private_browser_link_is_delivered_once_and_foreign_links_are_rejected() {
        let root =
            std::env::temp_dir().join(format!("auth-browser-{}", &random_id().unwrap()[..8]));
        fs::DirBuilder::new().mode(0o700).create(&root).unwrap();
        let home = root.join("home");
        let ui = root.join("ui");
        let bin = root.join("termux/bin");
        fs::DirBuilder::new().mode(0o700).create(&home).unwrap();
        fs::DirBuilder::new().mode(0o700).create(&ui).unwrap();
        fs::create_dir_all(&bin).unwrap();
        let captured = root.join("opened");
        let command = format!(
            "#!/bin/sh\nprintf '%s' \"$1\" > '{}.tmp'\nmv '{}.tmp' '{}'\n",
            captured.display(),
            captured.display(),
            captured.display()
        );
        write_private(&bin.join("termux-am-socket"), command.as_bytes(), true).unwrap();
        let bridge = AuthBrowser::start(&home, &ui, &root.join("termux")).unwrap();
        fs::write(
            ui.join("open-url"),
            "https://auth.openai.com/oauth/authorize?state=example",
        )
        .unwrap();
        tokio::time::timeout(Duration::from_secs(3), async {
            while !captured.exists() {
                tokio::time::sleep(Duration::from_millis(40)).await;
            }
        })
        .await
        .unwrap();
        let opened = fs::read_to_string(&captured).unwrap();
        assert!(opened.starts_with(
            "start -a android.intent.action.VIEW -d https://auth.openai.com/oauth/authorize"
        ));
        assert!(!ui.join("open-url").exists());
        fs::remove_file(&captured).unwrap();
        fs::write(ui.join("open-url"), "https://invalid.example/login").unwrap();
        tokio::time::sleep(Duration::from_millis(400)).await;
        assert!(!captured.exists(), "untrusted URL must never open Android");
        bridge.stop();
        drop(bridge);
        fs::remove_dir_all(root).unwrap();
    }
}
