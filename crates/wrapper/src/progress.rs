use crate::error::LocalProductError;
use crate::state::generation_state;
use std::io::Write as _;

const UPDATE_SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

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
    transient_worker: Option<UpdateTransientWorker>,
}

fn render_transient_frame(frame_index: usize, message: &str) {
    let frame = UPDATE_SPINNER_FRAMES[frame_index % UPDATE_SPINNER_FRAMES.len()];
    let mut stderr = std::io::stderr().lock();
    let _ = write!(stderr, "\r\x1b[2K{frame} {message}");
    let _ = stderr.flush();
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
            transient_worker: None,
        }
    }

    pub(super) fn transient(&mut self, message: &str) {
        if !self.enabled {
            return;
        }
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
        render_transient_frame(0, message);
        let (sender, receiver) = std::sync::mpsc::channel::<UpdateTransientCommand>();
        let initial_message = message.to_owned();
        let worker = std::thread::Builder::new()
            .name("codex-update-spinner".to_owned())
            .spawn(move || {
                let mut current_message = initial_message;
                let mut current_frame = 0usize;
                loop {
                    match receiver.recv_timeout(std::time::Duration::from_millis(80)) {
                        Ok(UpdateTransientCommand::Update { message }) => {
                            current_message = message;
                            render_transient_frame(current_frame, &current_message);
                        }
                        Ok(UpdateTransientCommand::Stop)
                        | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
                        Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                            current_frame = (current_frame + 1) % UPDATE_SPINNER_FRAMES.len();
                            render_transient_frame(current_frame, &current_message);
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

    fn clear_transient(&mut self) {
        if let Some(mut transient) = self.transient_worker.take() {
            let _ = transient.sender.send(UpdateTransientCommand::Stop);
            if let Some(worker) = transient.worker.take() {
                let _ = worker.join();
            }
        }
        if !self.transient_visible {
            return;
        }
        let mut stderr = std::io::stderr().lock();
        let _ = write!(stderr, "\r\x1b[2K");
        let _ = stderr.flush();
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
