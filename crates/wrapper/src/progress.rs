use crate::error::LocalProductError;
use crate::state::generation_state;
use std::io::{self, Read, Write};
use std::mem::MaybeUninit;
use std::os::fd::{AsRawFd, RawFd};
use std::time::{Duration, Instant};

const UPDATE_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
const SPINNER_TICK_INTERVAL: Duration = Duration::from_millis(120);

fn update_progress_enabled(stdout_is_terminal: bool, stderr_is_terminal: bool) -> bool {
    stdout_is_terminal && stderr_is_terminal
}

enum UpdateTransientCommand {
    Update { message: String },
    Stop,
}

struct UpdateTransientWorker {
    sender: std::sync::mpsc::Sender<UpdateTransientCommand>,
    worker: Option<std::thread::JoinHandle<()>>,
}

pub(super) struct UpdatePresentation {
    enabled: bool,
    transient_visible: bool,
    last_message: Option<String>,
    transient_worker: Option<UpdateTransientWorker>,
}

fn terminal_columns(fd: RawFd) -> usize {
    let mut size = MaybeUninit::<libc::winsize>::uninit();
    // SAFETY: ioctl initializes the winsize at this writable pointer on success.
    if unsafe { libc::ioctl(fd, libc::TIOCGWINSZ, size.as_mut_ptr()) } != 0 {
        return 0;
    }
    // SAFETY: TIOCGWINSZ succeeded.
    usize::from(unsafe { size.assume_init() }.ws_col)
}

fn char_display_width(ch: char) -> usize {
    // Reserve two cells for non-ASCII characters, including wide path names.
    if ch.is_ascii() {
        1
    } else {
        2
    }
}

fn transient_row_with_width(frame: &str, message: &str, columns: usize) -> (String, usize) {
    // Leave the last column unused to avoid a terminal's pending autowrap.
    let mut remaining = columns.saturating_sub(1);
    let mut row = String::new();
    let mut width = 0usize;
    for ch in frame.chars() {
        let w = char_display_width(ch);
        if w > remaining {
            return (String::new(), 0);
        }
        row.push(ch);
        remaining -= w;
        width += w;
    }
    for ch in std::iter::once(' ').chain(message.chars()) {
        let ch = if ch.is_control() { '?' } else { ch };
        let w = char_display_width(ch);
        if w > remaining {
            break;
        }
        row.push(ch);
        remaining -= w;
        width += w;
    }
    (row, width)
}

#[cfg(test)]
fn transient_row(frame_index: usize, message: &str, columns: usize) -> String {
    let frame = UPDATE_SPINNER_FRAMES[frame_index % UPDATE_SPINNER_FRAMES.len()];
    transient_row_with_width(frame, message, columns).0
}

#[derive(Default)]
struct TransientRenderer {
    last_columns: usize,
    last_width: usize,
    visible: bool,
}

impl TransientRenderer {
    fn next_frame_bytes(
        &mut self,
        frame_index: usize,
        message: &str,
        columns: usize,
    ) -> Option<String> {
        let frame = UPDATE_SPINNER_FRAMES[frame_index % UPDATE_SPINNER_FRAMES.len()];
        let (row, width) = transient_row_with_width(frame, message, columns);
        if row.is_empty() {
            self.last_columns = columns;
            if self.visible {
                self.visible = false;
                self.last_width = 0;
                return Some("\r\x1b[K".to_owned());
            }
            return None;
        }
        // Write the complete row in one atomic write(2) so the terminal never
        // paints a blank line between erase and text, and the cursor stays at
        // the end of the row instead of jumping back to column 1 on ticks.
        let need_eol_clear =
            !self.visible || width < self.last_width || columns != self.last_columns;
        let mut out = String::with_capacity(row.len() + 5);
        out.push('\r');
        out.push_str(&row);
        if need_eol_clear {
            out.push_str("\x1b[K");
        }
        self.visible = true;
        self.last_columns = columns;
        self.last_width = width;
        Some(out)
    }
}

fn write_stderr_atomic(bytes: &[u8]) {
    let mut stderr = io::stderr().lock();
    let _ = stderr.write_all(bytes);
    let _ = stderr.flush();
}

fn render_transient_with(
    renderer: &mut TransientRenderer,
    frame_index: usize,
    message: &str,
    columns: usize,
) {
    if let Some(output) = renderer.next_frame_bytes(frame_index, message, columns) {
        write_stderr_atomic(output.as_bytes());
    }
}

struct PromptTerminalMode {
    fd: RawFd,
    original: libc::termios,
}

impl PromptTerminalMode {
    fn enter(fd: RawFd) -> io::Result<Self> {
        let mut original = MaybeUninit::<libc::termios>::uninit();
        // SAFETY: tcgetattr writes one termios on success.
        if unsafe { libc::tcgetattr(fd, original.as_mut_ptr()) } != 0 {
            return Err(io::Error::last_os_error());
        }
        // SAFETY: tcgetattr succeeded.
        let original = unsafe { original.assume_init() };
        let mut prompt = original;
        // Read control keys ourselves so the old mode is restored before
        // delivering a keyboard signal to the process.
        prompt.c_lflag &= !(libc::ICANON | libc::ECHO | libc::ISIG);
        prompt.c_cc[libc::VMIN] = 1;
        prompt.c_cc[libc::VTIME] = 0;
        // SAFETY: prompt holds valid termios attributes for this open descriptor.
        if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &prompt) } != 0 {
            return Err(io::Error::last_os_error());
        }
        Ok(Self { fd, original })
    }
}

impl Drop for PromptTerminalMode {
    fn drop(&mut self) {
        // SAFETY: original came from this descriptor, which remains open.
        while unsafe { libc::tcsetattr(self.fd, libc::TCSANOW, &self.original) } != 0 {
            if io::Error::last_os_error().kind() != io::ErrorKind::Interrupted {
                break;
            }
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
enum UpdateReply {
    Accept,
    Decline,
    Signal(libc::c_int),
}

fn read_update_reply(
    input: &mut impl Read,
    control_reply: impl Fn(u8) -> Option<UpdateReply>,
) -> UpdateReply {
    let mut byte = [0];
    loop {
        match input.read_exact(&mut byte) {
            Ok(()) => match control_reply(byte[0]) {
                Some(reply) => return reply,
                None => match byte[0] {
                    b'y' | b'Y' => return UpdateReply::Accept,
                    b'n' | b'N' | b'\r' | b'\n' => return UpdateReply::Decline,
                    _ => continue,
                },
            },
            Err(_) => return UpdateReply::Decline,
        }
    }
}

pub(super) fn prompt_update_confirmation(
    input: &mut std::fs::File,
    err_out: &mut impl Write,
    prompt: &str,
) -> bool {
    let fd = input.as_raw_fd();
    // SAFETY: fpathconf reads a property of the open terminal descriptor.
    let disabled = unsafe { libc::fpathconf(fd, libc::_PC_VDISABLE) };
    loop {
        let Ok(tty) = PromptTerminalMode::enter(fd) else {
            return false;
        };
        if write!(err_out, "{prompt}")
            .and_then(|()| err_out.flush())
            .is_err()
        {
            return false;
        }
        let original = &tty.original;
        let reply = read_update_reply(input, |key| {
            if libc::c_long::from(key) == disabled {
                return None;
            }
            if original.c_lflag & libc::ISIG != 0 {
                for (index, signal) in [
                    (libc::VINTR, libc::SIGINT),
                    (libc::VQUIT, libc::SIGQUIT),
                    (libc::VSUSP, libc::SIGTSTP),
                ] {
                    if original.c_cc[index] == key {
                        return Some(UpdateReply::Signal(signal));
                    }
                }
            }
            (original.c_lflag & libc::ICANON != 0 && original.c_cc[libc::VEOF] == key)
                .then_some(UpdateReply::Decline)
        });
        drop(tty);
        let _ = writeln!(err_out);
        match reply {
            UpdateReply::Accept => return true,
            UpdateReply::Decline => return false,
            UpdateReply::Signal(signal) => {
                // SAFETY: this is a valid signal selected by a configured key.
                unsafe { libc::raise(signal) };
            }
        }
    }
}

impl UpdatePresentation {
    pub(super) fn new() -> Self {
        use std::io::IsTerminal as _;
        Self {
            enabled: update_progress_enabled(
                std::io::stdout().is_terminal(),
                std::io::stderr().is_terminal(),
            ),
            transient_visible: false,
            last_message: None,
            transient_worker: None,
        }
    }

    pub(super) fn transient(&mut self, message: &str) {
        if !self.enabled {
            return;
        }
        if self.last_message.as_deref() == Some(message) && self.transient_worker.is_some() {
            return;
        }
        self.last_message = Some(message.to_owned());
        if let Some(worker) = self.transient_worker.as_ref() {
            if worker
                .sender
                .send(UpdateTransientCommand::Update {
                    message: message.to_owned(),
                })
                .is_ok()
            {
                self.transient_visible = true;
                return;
            }
        }
        let stderr_fd = io::stderr().as_raw_fd();
        let mut renderer = TransientRenderer::default();
        render_transient_with(&mut renderer, 0, message, terminal_columns(stderr_fd));
        let (sender, receiver) = std::sync::mpsc::channel::<UpdateTransientCommand>();
        let initial_message = message.to_owned();
        let worker = std::thread::Builder::new()
            .name("codex-update-spinner".to_owned())
            .spawn(move || {
                let mut current_message = initial_message;
                let mut current_frame = 0usize;
                let mut next_tick = Instant::now() + SPINNER_TICK_INTERVAL;
                loop {
                    let wait = next_tick.saturating_duration_since(Instant::now());
                    match receiver.recv_timeout(wait) {
                        Ok(UpdateTransientCommand::Update { mut message }) => {
                            let mut stop = false;
                            while let Ok(cmd) = receiver.try_recv() {
                                match cmd {
                                    UpdateTransientCommand::Update { message: next } => {
                                        message = next;
                                    }
                                    UpdateTransientCommand::Stop => {
                                        stop = true;
                                        break;
                                    }
                                }
                            }
                            if stop {
                                break;
                            }
                            let now = Instant::now();
                            let ticked = now >= next_tick;
                            if ticked {
                                current_frame = (current_frame + 1) % UPDATE_SPINNER_FRAMES.len();
                                next_tick = now + SPINNER_TICK_INTERVAL;
                            }
                            let cols = terminal_columns(stderr_fd);
                            if message != current_message || cols != renderer.last_columns || ticked
                            {
                                current_message = message;
                                render_transient_with(
                                    &mut renderer,
                                    current_frame,
                                    &current_message,
                                    cols,
                                );
                            }
                        }
                        Ok(UpdateTransientCommand::Stop)
                        | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            current_frame = (current_frame + 1) % UPDATE_SPINNER_FRAMES.len();
                            next_tick = Instant::now() + SPINNER_TICK_INTERVAL;
                            let cols = terminal_columns(stderr_fd);
                            render_transient_with(
                                &mut renderer,
                                current_frame,
                                &current_message,
                                cols,
                            );
                        }
                    }
                }
            });
        if let Ok(worker) = worker {
            self.transient_worker = Some(UpdateTransientWorker {
                sender,
                worker: Some(worker),
            });
        }
        self.transient_visible = true;
    }

    pub(super) fn warn(&mut self, message: &str) {
        self.clear_transient();
        eprintln!("{message}");
    }

    fn clear_transient(&mut self) {
        if let Some(mut transient) = self.transient_worker.take() {
            let _ = transient.sender.send(UpdateTransientCommand::Stop);
            if let Some(worker) = transient.worker.take() {
                let _ = worker.join();
            }
        }
        self.last_message = None;
        if !self.transient_visible {
            return;
        }
        write_stderr_atomic(b"\r\x1b[2K");
        self.transient_visible = false;
    }

    pub(super) fn finish_activated(
        &mut self,
        previous_version: Option<&str>,
        current_version: &str,
        generation_id: &str,
    ) {
        self.clear_transient();
        match previous_version {
            Some(prev) if prev != current_version => {
                println!("Updated Codex {prev} -> {current_version} (generation {generation_id}).");
            }
            _ => {
                println!("Codex {current_version} is now active (generation {generation_id}).");
            }
        }
    }

    pub(super) fn finish_current(&mut self, current_version: &str) {
        self.clear_transient();
        println!("Codex {current_version} is already up to date.");
    }

    pub(super) fn finish_rollback(
        &mut self,
        previous_version: &str,
        current_version: &str,
        generation_id: &str,
    ) {
        self.clear_transient();
        if previous_version == current_version {
            println!("Rolled back Codex {current_version} (generation {generation_id}).");
        } else {
            println!("Rolled back Codex {previous_version} -> {current_version} (generation {generation_id}).");
        }
    }

    pub(super) fn fail(&mut self, error: &LocalProductError) -> i32 {
        self.clear_transient();
        eprintln!("codex update: {error}");
        if matches!(
            error,
            LocalProductError::State(generation_state::ActivationTransactionError::CommitSync(_))
        ) {
            3
        } else {
            1
        }
    }
}

impl Drop for UpdatePresentation {
    fn drop(&mut self) {
        self.clear_transient();
    }
}

pub(super) fn format_bytes(bytes: u64) -> String {
    const KIB: f64 = 1024.0;
    const MIB: f64 = 1024.0 * 1024.0;
    const GIB: f64 = 1024.0 * 1024.0 * 1024.0;
    let b = bytes as f64;
    if b >= GIB {
        format!("{:.2} GiB", b / GIB)
    } else if b >= MIB {
        format!("{:.1} MiB", b / MIB)
    } else if b >= KIB {
        format!("{:.1} KiB", b / KIB)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transient_rows_reserve_the_last_column_and_escape_controls() {
        let path =
            "/data/data/com.termux/files/home/.local/share/codex/manager/profiles/가나다/home";
        let message = format!("Restarting managed daemon in {path} (Codex 0.171.0)...");
        let row = transient_row(0, &message, 44);
        assert_eq!(row, "⠋ Restarting managed daemon in /data/data/");
        assert_eq!(transient_row(0, "가나다\n", 10), "⠋ 가나다");
        assert_eq!(transient_row(0, "one\ntwo\x1b", 40), "⠋ one?two?");
        assert_eq!(transient_row(0, &message, 2), "");
        assert_eq!(transient_row(0, &message, 0), "");
    }

    #[test]
    fn transient_renderer_keeps_cursor_at_row_end_without_full_line_erase() {
        let mut renderer = TransientRenderer::default();
        // Initial frame writes text then clears trailing EOL in one atomic buffer.
        assert_eq!(
            renderer.next_frame_bytes(0, "Checking official Codex release...", 42),
            Some("\r⠋ Checking official Codex release...\x1b[K".to_owned())
        );
        // Same message & width: atomic full-row overwrite keeps cursor at end with zero erase.
        assert_eq!(
            renderer.next_frame_bytes(1, "Checking official Codex release...", 42),
            Some("\r⠙ Checking official Codex release...".to_owned())
        );
        // Longer message at same width overwrites everything: no EOL clear needed.
        assert_eq!(
            renderer.next_frame_bytes(2, "Downloading Codex 0.171.0 (12.0 MiB / 191.0 MiB)...", 80),
            Some("\r⠹ Downloading Codex 0.171.0 (12.0 MiB / 191.0 MiB)...\x1b[K".to_owned())
        );
        assert_eq!(
            renderer.next_frame_bytes(
                3,
                "Downloading Codex 0.171.0 (120.0 MiB / 191.0 MiB)...",
                80
            ),
            Some("\r⠸ Downloading Codex 0.171.0 (120.0 MiB / 191.0 MiB)...".to_owned())
        );
        // Shorter message clears only trailing cells after the new text.
        assert_eq!(
            renderer.next_frame_bytes(4, "Activating Codex 0.171.0...", 80),
            Some("\r⠼ Activating Codex 0.171.0...\x1b[K".to_owned())
        );
        // Terminal shrinks so message truncates: full-row redraw at new width with EOL clear.
        assert_eq!(
            renderer.next_frame_bytes(5, "Activating Codex 0.171.0...", 20),
            Some("\r⠴ Activating Codex\x1b[K".to_owned())
        );
        // Next tick at same narrow width keeps cursor at end without EOL clear.
        assert_eq!(
            renderer.next_frame_bytes(6, "Activating Codex 0.171.0...", 20),
            Some("\r⠦ Activating Codex".to_owned())
        );
        // Too narrow to render even the spinner: clear once, then stay silent.
        assert_eq!(
            renderer.next_frame_bytes(7, "Activating Codex 0.171.0...", 2),
            Some("\r\x1b[K".to_owned())
        );
        assert_eq!(
            renderer.next_frame_bytes(8, "Activating Codex 0.171.0...", 2),
            None
        );
    }

    #[test]
    fn update_reply_uses_one_key_and_preserves_following_input() {
        for (keys, expected) in [
            (&b"yTUI"[..], UpdateReply::Accept),
            (&b"?YTUI"[..], UpdateReply::Accept),
            (&b"nTUI"[..], UpdateReply::Decline),
            (&b"NTUI"[..], UpdateReply::Decline),
            (&b"\rTUI"[..], UpdateReply::Decline),
            (&b"\nTUI"[..], UpdateReply::Decline),
        ] {
            let mut input = keys;
            assert_eq!(read_update_reply(&mut input, |_| None), expected);
            assert_eq!(input, b"TUI");
        }
        assert_eq!(
            read_update_reply(&mut &b""[..], |_| None),
            UpdateReply::Decline
        );
    }
}
