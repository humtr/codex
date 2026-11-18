#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ReleasePublicKey(pub(super) [u8; 32]);

impl ReleasePublicKey {
    pub(super) const LOCAL_ZERO: Self = Self([0u8; 32]);

    pub(super) fn parse_hex(value: &str) -> Option<Self> {
        if value.len() != 64 {
            return None;
        }
        fn nibble(byte: u8) -> Option<u8> {
            match byte {
                b'0'..=b'9' => Some(byte - b'0'),
                b'a'..=b'f' => Some(byte - b'a' + 10),
                _ => None,
            }
        }
        let bytes = value.as_bytes();
        let mut raw = [0u8; 32];
        for (index, output) in raw.iter_mut().enumerate() {
            *output = (nibble(bytes[index * 2])? << 4) | nibble(bytes[index * 2 + 1])?;
        }
        Some(Self(raw))
    }

    pub(super) fn to_hex(self) -> String {
        let mut value = String::with_capacity(64);
        for byte in self.0 {
            use std::fmt::Write as _;
            write!(&mut value, "{byte:02x}").expect("writing into String cannot fail");
        }
        value
    }
}

/// Transactional pointer-state persistence for activated Codex generations.
pub(super) mod generation_state {
    use super::ReleasePublicKey;
    use crate::{flock, FLOCK_EX, FLOCK_NB, FLOCK_UN};
    use std::fs;
    use std::io::{self, Read, Write};
    use std::os::fd::AsRawFd;
    use std::os::unix::fs::MetadataExt;
    use std::path::{Path, PathBuf};

    const GENERATION_ID_MAX_BYTES: usize = 512;
    const STATE_FILE_MAX_BYTES: usize = 16 * 1024;
    const STATE_FORMAT: &str = "codex-activation-state";
    const JOURNAL_FORMAT: &str = "codex-activation-journal";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct WrapperStatePaths {
        pub(crate) root: std::path::PathBuf,
        pub(crate) activation_state: std::path::PathBuf,
        pub(crate) activation_journal: std::path::PathBuf,
        pub(crate) activation_journal_temp: std::path::PathBuf,
        pub(crate) activation_state_temp: std::path::PathBuf,
        generation_root: Option<PathBuf>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct GenerationPointerState {
        pub(crate) update_key: ReleasePublicKey,
        pub(crate) current: String,
        pub(crate) current_key: ReleasePublicKey,
        pub(crate) previous: Option<String>,
        pub(crate) previous_key: Option<ReleasePublicKey>,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct ActivationJournal {
        pub(crate) operation: ActivationOperation,
        pub(crate) before: Option<GenerationPointerState>,
        pub(crate) after: GenerationPointerState,
    }

    impl ActivationJournal {
        pub(crate) fn is_rollback_transition(&self) -> bool {
            self.before.as_ref().is_some_and(|before| {
                plan_rollback_pointer_state(before).as_ref() == Ok(&self.after)
            })
        }
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub(crate) enum ActivationOperation {
        Update,
        Rollback,
        Legacy,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) enum StateFormatError {
        EmptyRoot,
        RelativeRoot,
        NulRoot,
        EmptyIdentity(&'static str),
        IdentityTooLong(&'static str),
        IdentityControl(&'static str),
        FileTooLarge(&'static str),
        InvalidUtf8(&'static str),
        MissingFinalNewline(&'static str),
        InvalidRecordCount(&'static str),
        InvalidField(&'static str),
        InvalidPresence(&'static str),
        InconsistentAbsent(&'static str),
        AmbiguousJournal,
        NoRollbackGeneration,
        NoChange,
    }

    impl std::fmt::Display for StateFormatError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                StateFormatError::EmptyRoot => f.write_str("wrapper state root is empty"),
                StateFormatError::RelativeRoot => {
                    f.write_str("wrapper state root must be an absolute path")
                }
                StateFormatError::NulRoot => f.write_str("wrapper state root contains NUL"),
                StateFormatError::EmptyIdentity(field) => {
                    write!(f, "generation identity '{field}' is empty")
                }
                StateFormatError::IdentityTooLong(field) => {
                    write!(f, "generation identity '{field}' exceeds the size limit")
                }
                StateFormatError::IdentityControl(field) => write!(
                    f,
                    "generation identity '{field}' is not a safe path component"
                ),
                StateFormatError::FileTooLarge(label) => {
                    write!(f, "{label} exceeds the bounded state-file size")
                }
                StateFormatError::InvalidUtf8(label) => {
                    write!(f, "{label} is not valid UTF-8")
                }
                StateFormatError::MissingFinalNewline(label) => {
                    write!(f, "{label} is missing its canonical final newline")
                }
                StateFormatError::InvalidRecordCount(label) => {
                    write!(f, "{label} has an invalid record count")
                }
                StateFormatError::InvalidField(label) => {
                    write!(f, "{label} has an invalid or out-of-order field")
                }
                StateFormatError::InvalidPresence(label) => {
                    write!(f, "{label} has an invalid presence marker")
                }
                StateFormatError::InconsistentAbsent(label) => {
                    write!(f, "{label} encodes data for an absent value")
                }
                StateFormatError::AmbiguousJournal => {
                    f.write_str("activation journal before/after states are identical")
                }
                StateFormatError::NoRollbackGeneration => {
                    f.write_str("activation state has no rollback generation")
                }
                StateFormatError::NoChange => {
                    f.write_str("activation transition would not change the current generation")
                }
            }
        }
    }

    impl std::error::Error for StateFormatError {}

    #[derive(Debug)]
    pub(crate) enum ActivationTransactionError {
        Format(StateFormatError),
        Io {
            operation: &'static str,
            source: std::io::Error,
        },
        UnsafeFileType(&'static str),
        StaleAuthoritativeState,
        PendingJournal,
        OrphanJournalTemporary,
        OrphanTemporaryState,
        RecoveryConflict,
        CommitSync(io::Error),
        CommittedCleanup {
            operation: &'static str,
            source: io::Error,
        },
        WriterBusy,
    }

    impl std::fmt::Display for ActivationTransactionError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                ActivationTransactionError::Format(err) => err.fmt(f),
                ActivationTransactionError::Io { operation, source } => {
                    write!(f, "{operation} failed: {source}")
                }
                ActivationTransactionError::UnsafeFileType(label) => {
                    write!(f, "{label} has an unsafe file type")
                }
                ActivationTransactionError::StaleAuthoritativeState => f.write_str(
                    "authoritative activation state does not match expected before state",
                ),
                ActivationTransactionError::PendingJournal => {
                    f.write_str("activation journal already exists; recovery is required")
                }
                ActivationTransactionError::OrphanJournalTemporary => {
                    f.write_str("orphan activation-journal temporary exists")
                }
                ActivationTransactionError::OrphanTemporaryState => f.write_str(
                    "orphan activation-state temporary exists without recoverable ownership",
                ),
                ActivationTransactionError::RecoveryConflict => f.write_str(
                    "activation recovery cannot match authoritative state to journal before/after",
                ),
                ActivationTransactionError::CommitSync(err) => write!(
                    f,
                    "activation state has been published but could not be synchronized: {err}; check 'codex wrapper status' before issuing another update or rollback",
                ),
                ActivationTransactionError::CommittedCleanup { operation, source } => {
                    write!(f, "activation committed; {operation} failed: {source}")
                }
                ActivationTransactionError::WriterBusy => {
                    f.write_str("another activation transaction is in progress; retry later")
                }
            }
        }
    }

    impl std::error::Error for ActivationTransactionError {
        fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
            match self {
                ActivationTransactionError::Format(err) => Some(err),
                ActivationTransactionError::Io { source, .. }
                | ActivationTransactionError::CommittedCleanup { source, .. }
                | ActivationTransactionError::CommitSync(source) => Some(source),
                _ => None,
            }
        }
    }

    impl From<StateFormatError> for ActivationTransactionError {
        fn from(value: StateFormatError) -> Self {
            Self::Format(value)
        }
    }

    fn io_error(operation: &'static str, source: std::io::Error) -> ActivationTransactionError {
        ActivationTransactionError::Io { operation, source }
    }

    #[derive(Debug)]
    pub(crate) struct ActivationLockGuard {
        file: Option<std::fs::File>,
    }

    impl Drop for ActivationLockGuard {
        fn drop(&mut self) {
            if let Some(file) = self.file.as_ref() {
                let _ = unsafe { flock(file.as_raw_fd(), FLOCK_UN) };
            }
        }
    }

    fn acquire_activation_lock_with_mode(
        paths: &WrapperStatePaths,
        mode: std::os::raw::c_int,
        wait_timeout: Option<std::time::Duration>,
    ) -> Result<ActivationLockGuard, ActivationTransactionError> {
        let metadata = match std::fs::symlink_metadata(&paths.root) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                return Ok(ActivationLockGuard { file: None })
            }
            Err(source) => {
                return Err(io_error(
                    "inspect wrapper state root for writer lock",
                    source,
                ))
            }
        };
        if !metadata.file_type().is_dir() {
            return Err(ActivationTransactionError::UnsafeFileType(
                "wrapper state root",
            ));
        }
        let file = std::fs::File::open(&paths.root)
            .map_err(|source| io_error("open wrapper state root for writer lock", source))?;
        let deadline = wait_timeout.map(|timeout| std::time::Instant::now() + timeout);
        loop {
            let result = unsafe { flock(file.as_raw_fd(), mode | FLOCK_NB) };
            if result == 0 {
                return Ok(ActivationLockGuard { file: Some(file) });
            }
            let source = std::io::Error::last_os_error();
            if source.kind() == std::io::ErrorKind::WouldBlock {
                if deadline.is_some_and(|until| std::time::Instant::now() < until) {
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    continue;
                }
                return Err(ActivationTransactionError::WriterBusy);
            }
            return Err(io_error("acquire wrapper state writer lock", source));
        }
    }

    pub(crate) fn acquire_activation_lock(
        paths: &WrapperStatePaths,
    ) -> Result<ActivationLockGuard, ActivationTransactionError> {
        acquire_activation_lock_with_mode(paths, FLOCK_EX, None)
    }

    pub(crate) fn acquire_activation_read_lock(
        paths: &WrapperStatePaths,
    ) -> Result<ActivationLockGuard, ActivationTransactionError> {
        acquire_activation_lock_with_mode(
            paths,
            crate::fs_util::LOCK_SH,
            Some(std::time::Duration::from_secs(2)),
        )
    }

    impl WrapperStatePaths {
        pub(crate) fn new(root: &std::path::Path) -> Result<Self, StateFormatError> {
            use std::os::unix::ffi::OsStrExt;

            if root.as_os_str().is_empty() {
                return Err(StateFormatError::EmptyRoot);
            }
            if root.as_os_str().as_bytes().contains(&0) {
                return Err(StateFormatError::NulRoot);
            }
            if !root.is_absolute() {
                return Err(StateFormatError::RelativeRoot);
            }
            Ok(Self {
                root: root.to_path_buf(),
                activation_state: root.join("activation-state"),
                activation_journal: root.join("activation-journal"),
                activation_journal_temp: root.join("activation-journal.tmp"),
                activation_state_temp: root.join("activation-state.tmp"),
                generation_root: None,
            })
        }
        pub(crate) fn with_generation_root(mut self, root: &Path) -> Self {
            self.generation_root = Some(root.to_path_buf());
            self
        }

        pub(crate) fn standalone_current_target(&self, generation: &str) -> Option<PathBuf> {
            Some(Path::new(self.generation_root.as_ref()?.file_name()?).join(generation))
        }

        pub(crate) fn read_standalone_current(
            &self,
        ) -> Result<Option<PathBuf>, ActivationTransactionError> {
            let Some(root) = self.standalone_root() else {
                return Ok(None);
            };
            read_symlink(
                &root.join("current"),
                "standalone current pointer (expected a symlink)",
            )
        }

        fn standalone_root(&self) -> Option<&Path> {
            self.generation_root.as_ref()?.parent()
        }
    }

    pub(crate) fn prepare_wrapper_state_paths(
        paths: &WrapperStatePaths,
    ) -> Result<(), ActivationTransactionError> {
        crate::fs_util::private_dir(&paths.root)
            .map_err(|err| io_error("prepare wrapper state root", err))?;
        let directory = std::fs::File::open(&paths.root)
            .map_err(|err| io_error("open wrapper state root for sync", err))?;
        directory
            .sync_all()
            .map_err(|err| io_error("sync wrapper state root", err))?;
        Ok(())
    }

    pub(crate) fn validate_generation_identity(
        value: &str,
        field: &'static str,
    ) -> Result<(), StateFormatError> {
        if value.is_empty() {
            return Err(StateFormatError::EmptyIdentity(field));
        }
        if value.len() > GENERATION_ID_MAX_BYTES {
            return Err(StateFormatError::IdentityTooLong(field));
        }
        if value == "."
            || value == ".."
            || value
                .as_bytes()
                .iter()
                .any(|byte| *byte == b'/' || byte.is_ascii_control())
        {
            return Err(StateFormatError::IdentityControl(field));
        }
        Ok(())
    }

    fn validate_pointer_state(state: &GenerationPointerState) -> Result<(), StateFormatError> {
        validate_generation_identity(&state.current, "current")?;
        if state.previous.is_some() != state.previous_key.is_some() {
            return Err(StateFormatError::InconsistentAbsent(
                "previous verifier key",
            ));
        }
        if let Some(previous) = state.previous.as_deref() {
            validate_generation_identity(previous, "previous")?;
            if previous == state.current {
                return Err(StateFormatError::NoChange);
            }
        }
        Ok(())
    }

    pub(crate) fn plan_initial_pointer_state(
        complete_candidate_identity: &str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        Ok(GenerationPointerState {
            update_key: ReleasePublicKey::LOCAL_ZERO,
            current: complete_candidate_identity.to_owned(),
            current_key: ReleasePublicKey::LOCAL_ZERO,
            previous: None,
            previous_key: None,
        })
    }

    pub(crate) fn plan_activation_pointer_state(
        before: &GenerationPointerState,
        complete_candidate_identity: &str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        validate_generation_identity(complete_candidate_identity, "candidate")?;
        if before.current == complete_candidate_identity {
            return Err(StateFormatError::NoChange);
        }
        Ok(GenerationPointerState {
            update_key: before.update_key,
            current: complete_candidate_identity.to_owned(),
            current_key: ReleasePublicKey::LOCAL_ZERO,
            previous: Some(before.current.clone()),
            previous_key: Some(before.current_key),
        })
    }

    pub(crate) fn plan_rollback_pointer_state(
        before: &GenerationPointerState,
    ) -> Result<GenerationPointerState, StateFormatError> {
        validate_pointer_state(before)?;
        let previous = before
            .previous
            .as_deref()
            .ok_or(StateFormatError::NoRollbackGeneration)?;
        let previous_key = before
            .previous_key
            .ok_or(StateFormatError::NoRollbackGeneration)?;
        Ok(GenerationPointerState {
            update_key: before.update_key,
            current: previous.to_owned(),
            current_key: previous_key,
            previous: Some(before.current.clone()),
            previous_key: Some(before.current_key),
        })
    }

    pub(crate) fn encode_pointer_state(
        state: &GenerationPointerState,
    ) -> Result<Vec<u8>, StateFormatError> {
        validate_pointer_state(state)?;
        let (previous_present, previous, previous_key) =
            match (state.previous.as_deref(), state.previous_key) {
                (Some(previous), Some(previous_key)) => ("1", previous, previous_key.to_hex()),
                (None, None) => ("0", "", String::new()),
                _ => {
                    return Err(StateFormatError::InconsistentAbsent(
                        "previous verifier key",
                    ))
                }
            };
        Ok(format!(
            "format={STATE_FORMAT}\nupdate_key={}\ncurrent={}\ncurrent_key={}\nprevious_present={previous_present}\nprevious={previous}\nprevious_key={previous_key}\n",
            state.update_key.to_hex(),
            state.current,
            state.current_key.to_hex(),
        )
        .into_bytes())
    }

    fn parse_lines<'a>(
        bytes: &'a [u8],
        label: &'static str,
    ) -> Result<Vec<&'a str>, StateFormatError> {
        if bytes.len() > STATE_FILE_MAX_BYTES {
            return Err(StateFormatError::FileTooLarge(label));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| StateFormatError::InvalidUtf8(label))?;
        if !text.ends_with('\n') {
            return Err(StateFormatError::MissingFinalNewline(label));
        }
        let body = &text[..text.len() - 1];
        Ok(body.split('\n').collect())
    }

    fn parse_field<'a>(
        line: &'a str,
        prefix: &str,
        label: &'static str,
    ) -> Result<&'a str, StateFormatError> {
        line.strip_prefix(prefix)
            .ok_or(StateFormatError::InvalidField(label))
    }

    fn parse_presence(value: &str, label: &'static str) -> Result<bool, StateFormatError> {
        match value {
            "0" => Ok(false),
            "1" => Ok(true),
            _ => Err(StateFormatError::InvalidPresence(label)),
        }
    }

    fn parse_key(value: &str, label: &'static str) -> Result<ReleasePublicKey, StateFormatError> {
        ReleasePublicKey::parse_hex(value).ok_or(StateFormatError::InvalidField(label))
    }

    fn parse_pointer_values(
        update_key: &str,
        current: &str,
        current_key: &str,
        previous_present: &str,
        previous: &str,
        previous_key: &str,
        label: &'static str,
    ) -> Result<GenerationPointerState, StateFormatError> {
        let has_previous = parse_presence(previous_present, label)?;
        if !has_previous && (!previous.is_empty() || !previous_key.is_empty()) {
            return Err(StateFormatError::InconsistentAbsent(label));
        }
        if has_previous && (previous.is_empty() || previous_key.is_empty()) {
            return Err(StateFormatError::InconsistentAbsent(label));
        }
        let state = GenerationPointerState {
            update_key: parse_key(update_key, "update key")?,
            current: current.to_owned(),
            current_key: parse_key(current_key, "current verifier key")?,
            previous: has_previous.then(|| previous.to_owned()),
            previous_key: if has_previous {
                Some(parse_key(previous_key, "previous verifier key")?)
            } else {
                None
            },
        };
        validate_pointer_state(&state)?;
        Ok(state)
    }

    pub(crate) fn parse_pointer_state(
        bytes: &[u8],
    ) -> Result<GenerationPointerState, StateFormatError> {
        let records = parse_lines(bytes, "activation state")?;
        if records.len() != 7 {
            return Err(StateFormatError::InvalidRecordCount("activation state"));
        }
        if records[0] != format!("format={STATE_FORMAT}") {
            return Err(StateFormatError::InvalidField("activation state format"));
        }
        let update_key = parse_field(records[1], "update_key=", "activation state update key")?;
        let current = parse_field(records[2], "current=", "activation state current")?;
        let current_key = parse_field(records[3], "current_key=", "activation state current key")?;
        let previous_present = parse_field(
            records[4],
            "previous_present=",
            "activation state previous presence",
        )?;
        let previous = parse_field(records[5], "previous=", "activation state previous")?;
        let previous_key =
            parse_field(records[6], "previous_key=", "activation state previous key")?;
        parse_pointer_values(
            update_key,
            current,
            current_key,
            previous_present,
            previous,
            previous_key,
            "activation state previous",
        )
    }

    pub(crate) fn encode_activation_journal(
        journal: &ActivationJournal,
    ) -> Result<Vec<u8>, StateFormatError> {
        if journal.before.as_ref() == Some(&journal.after) {
            return Err(StateFormatError::AmbiguousJournal);
        }
        if let Some(before) = journal.before.as_ref() {
            validate_pointer_state(before)?;
        }
        validate_pointer_state(&journal.after)?;

        let (
            before_present,
            before_update_key,
            before_current,
            before_current_key,
            before_previous_present,
            before_previous,
            before_previous_key,
        ) = match journal.before.as_ref() {
            Some(before) => {
                let (previous_present, previous, previous_key) =
                    match (before.previous.as_deref(), before.previous_key) {
                        (Some(previous), Some(previous_key)) => {
                            ("1", previous, previous_key.to_hex())
                        }
                        (None, None) => ("0", "", String::new()),
                        _ => {
                            return Err(StateFormatError::InconsistentAbsent(
                                "journal before previous",
                            ))
                        }
                    };
                (
                    "1",
                    before.update_key.to_hex(),
                    before.current.as_str(),
                    before.current_key.to_hex(),
                    previous_present,
                    previous,
                    previous_key,
                )
            }
            None => (
                "0",
                String::new(),
                "",
                String::new(),
                "0",
                "",
                String::new(),
            ),
        };
        let (after_previous_present, after_previous, after_previous_key) = match (
            journal.after.previous.as_deref(),
            journal.after.previous_key,
        ) {
            (Some(previous), Some(previous_key)) => ("1", previous, previous_key.to_hex()),
            (None, None) => ("0", "", String::new()),
            _ => {
                return Err(StateFormatError::InconsistentAbsent(
                    "journal after previous",
                ))
            }
        };
        let operation = match journal.operation {
            ActivationOperation::Update => "operation=update\n",
            ActivationOperation::Rollback => "operation=rollback\n",
            ActivationOperation::Legacy => "",
        };
        Ok(format!(
            "format={JOURNAL_FORMAT}\nbefore_present={before_present}\nbefore_update_key={before_update_key}\nbefore_current={before_current}\nbefore_current_key={before_current_key}\nbefore_previous_present={before_previous_present}\nbefore_previous={before_previous}\nbefore_previous_key={before_previous_key}\nafter_update_key={}\nafter_current={}\nafter_current_key={}\nafter_previous_present={after_previous_present}\nafter_previous={after_previous}\nafter_previous_key={after_previous_key}\n{operation}",
            journal.after.update_key.to_hex(),
            journal.after.current,
            journal.after.current_key.to_hex(),
        )
        .into_bytes())
    }

    pub(crate) fn parse_activation_journal(
        bytes: &[u8],
    ) -> Result<ActivationJournal, StateFormatError> {
        let records = parse_lines(bytes, "activation journal")?;
        let operation = match records.len() {
            14 => ActivationOperation::Legacy,
            15 => match records[14] {
                "operation=update" => ActivationOperation::Update,
                "operation=rollback" => ActivationOperation::Rollback,
                _ => {
                    return Err(StateFormatError::InvalidField(
                        "activation journal operation",
                    ))
                }
            },
            _ => return Err(StateFormatError::InvalidRecordCount("activation journal")),
        };
        if records[0] != format!("format={JOURNAL_FORMAT}") {
            return Err(StateFormatError::InvalidField("activation journal format"));
        }
        let before_present = parse_presence(
            parse_field(records[1], "before_present=", "journal before presence")?,
            "journal before presence",
        )?;
        let before_update_key = parse_field(
            records[2],
            "before_update_key=",
            "journal before update key",
        )?;
        let before_current = parse_field(records[3], "before_current=", "journal before current")?;
        let before_current_key = parse_field(
            records[4],
            "before_current_key=",
            "journal before current key",
        )?;
        let before_previous_present = parse_field(
            records[5],
            "before_previous_present=",
            "journal before previous presence",
        )?;
        let before_previous =
            parse_field(records[6], "before_previous=", "journal before previous")?;
        let before_previous_key = parse_field(
            records[7],
            "before_previous_key=",
            "journal before previous key",
        )?;
        let before = if before_present {
            Some(parse_pointer_values(
                before_update_key,
                before_current,
                before_current_key,
                before_previous_present,
                before_previous,
                before_previous_key,
                "journal before state",
            )?)
        } else {
            if !before_update_key.is_empty()
                || !before_current.is_empty()
                || !before_current_key.is_empty()
                || before_previous_present != "0"
                || !before_previous.is_empty()
                || !before_previous_key.is_empty()
            {
                return Err(StateFormatError::InconsistentAbsent("journal before state"));
            }
            None
        };

        let after_update_key =
            parse_field(records[8], "after_update_key=", "journal after update key")?;
        let after_current = parse_field(records[9], "after_current=", "journal after current")?;
        let after_current_key = parse_field(
            records[10],
            "after_current_key=",
            "journal after current key",
        )?;
        let after_previous_present = parse_field(
            records[11],
            "after_previous_present=",
            "journal after previous presence",
        )?;
        let after_previous = parse_field(records[12], "after_previous=", "journal after previous")?;
        let after_previous_key = parse_field(
            records[13],
            "after_previous_key=",
            "journal after previous key",
        )?;
        let after = parse_pointer_values(
            after_update_key,
            after_current,
            after_current_key,
            after_previous_present,
            after_previous,
            after_previous_key,
            "journal after state",
        )?;
        if before.as_ref() == Some(&after) {
            return Err(StateFormatError::AmbiguousJournal);
        }
        let journal = ActivationJournal {
            operation,
            before,
            after,
        };
        if operation == ActivationOperation::Rollback && !journal.is_rollback_transition() {
            return Err(StateFormatError::InvalidField(
                "activation journal rollback",
            ));
        }
        Ok(journal)
    }

    pub(crate) trait ActivationIo {
        fn write_new_synced(&mut self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()>;
        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()>;
        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()>;
        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()>;
        fn symlink(&mut self, target: &Path, path: &Path) -> io::Result<()> {
            std::os::unix::fs::symlink(target, path)
        }
    }

    pub(crate) struct FsActivationIo;

    impl ActivationIo for FsActivationIo {
        fn write_new_synced(&mut self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)?;
            file.write_all(data)?;
            file.sync_all()
        }

        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            std::fs::File::open(path)?.sync_all()
        }

        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()> {
            std::fs::rename(from, to)
        }

        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()> {
            std::fs::remove_file(path)
        }
    }

    fn read_symlink(
        path: &Path,
        label: &'static str,
    ) -> Result<Option<PathBuf>, ActivationTransactionError> {
        match fs::symlink_metadata(path) {
            Ok(meta) if meta.is_symlink() => fs::read_link(path)
                .map(Some)
                .map_err(|err| io_error("read standalone symlink", err)),
            Ok(_) => Err(ActivationTransactionError::UnsafeFileType(label)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(err) => Err(io_error("inspect standalone symlink", err)),
        }
    }

    // The activation lock also serializes replacement and cleanup of this link.
    pub(crate) fn sync_standalone_current<I: ActivationIo>(
        paths: &WrapperStatePaths,
        generation: Option<&str>,
        journal: Option<&ActivationJournal>,
        io: &mut I,
        _lock: &ActivationLockGuard,
    ) -> Result<(), ActivationTransactionError> {
        let Some(root) = paths.standalone_root() else {
            return Ok(());
        };
        let current = paths.read_standalone_current()?;
        let desired = generation.and_then(|id| paths.standalone_current_target(id));
        let attempted = journal.and_then(|j| paths.standalone_current_target(&j.after.current));
        let tmp = root.join(".current.wrapper.tmp");
        let staged = read_symlink(&tmp, "standalone temporary current pointer")?;
        if current == desired && journal.is_none() && staged.is_none() {
            return Ok(());
        }

        let uid = crate::fs_util::current_uid()
            .map_err(|err| io_error("get standalone directory owner", err))?;
        for dir in [root, paths.generation_root.as_deref().unwrap()] {
            let meta = fs::symlink_metadata(dir)
                .map_err(|err| io_error("inspect standalone directory", err))?;
            if !meta.is_dir() || meta.uid() != uid {
                return Err(ActivationTransactionError::UnsafeFileType(
                    "standalone directory",
                ));
            }
        }

        // A crash may leave the staged symlink after creation and before rename.
        // A temporary that is not a symlink is rejected by read_symlink above.
        if staged.is_some() {
            io.remove_file(&tmp)
                .map_err(|err| io_error("remove stale standalone current temporary", err))?;
        }
        let link = root.join("current");
        if current != desired {
            if let Some(target) = desired {
                io.symlink(&target, &tmp)
                    .map_err(|err| io_error("create temporary standalone current symlink", err))?;
                io.rename(&tmp, &link)
                    .map_err(|err| io_error("publish standalone current symlink", err))?;
            } else if current == attempted {
                io.remove_file(&link).map_err(|err| {
                    io_error("remove uncommitted standalone current symlink", err)
                })?;
            }
        }
        io.sync_dir(root)
            .map_err(|err| io_error("sync standalone package directory", err))
    }

    fn path_exists(
        path: &std::path::Path,
        operation: &'static str,
    ) -> Result<bool, ActivationTransactionError> {
        match std::fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(err) => Err(io_error(operation, err)),
        }
    }

    fn read_bounded_regular_file(
        path: &std::path::Path,
        label: &'static str,
    ) -> Result<Option<Vec<u8>>, ActivationTransactionError> {
        let metadata = match std::fs::symlink_metadata(path) {
            Ok(metadata) => metadata,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(err) => return Err(io_error("inspect activation state file", err)),
        };
        if !metadata.file_type().is_file() {
            return Err(ActivationTransactionError::UnsafeFileType(label));
        }
        if metadata.len() > STATE_FILE_MAX_BYTES as u64 {
            return Err(StateFormatError::FileTooLarge(label).into());
        }
        let file =
            std::fs::File::open(path).map_err(|err| io_error("open activation state file", err))?;
        let mut bytes = Vec::new();
        file.take((STATE_FILE_MAX_BYTES + 1) as u64)
            .read_to_end(&mut bytes)
            .map_err(|err| io_error("read activation state file", err))?;
        if bytes.len() > STATE_FILE_MAX_BYTES {
            return Err(StateFormatError::FileTooLarge(label).into());
        }
        Ok(Some(bytes))
    }

    pub(crate) fn read_pointer_state(
        paths: &WrapperStatePaths,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        read_bounded_regular_file(&paths.activation_state, "activation state")?
            .map(|bytes| parse_pointer_state(&bytes).map_err(ActivationTransactionError::from))
            .transpose()
    }

    fn read_activation_journal(
        paths: &WrapperStatePaths,
    ) -> Result<Option<ActivationJournal>, ActivationTransactionError> {
        read_bounded_regular_file(&paths.activation_journal, "activation journal")?
            .map(|bytes| parse_activation_journal(&bytes).map_err(ActivationTransactionError::from))
            .transpose()
    }

    pub(crate) fn activate_pointer_state_locked<I: ActivationIo>(
        paths: &WrapperStatePaths,
        operation: ActivationOperation,
        before: Option<&GenerationPointerState>,
        after: &GenerationPointerState,
        io: &mut I,
        lock: &ActivationLockGuard,
    ) -> Result<(), ActivationTransactionError> {
        if let Some(before) = before {
            validate_pointer_state(before)?;
        }
        validate_pointer_state(after)?;
        if before == Some(after) {
            return Err(StateFormatError::AmbiguousJournal.into());
        }
        let authoritative = read_pointer_state(paths)?;
        if authoritative.as_ref() != before {
            return Err(ActivationTransactionError::StaleAuthoritativeState);
        }
        if path_exists(&paths.activation_journal, "inspect activation journal")? {
            return Err(ActivationTransactionError::PendingJournal);
        }
        if path_exists(
            &paths.activation_journal_temp,
            "inspect activation journal temporary",
        )? {
            return Err(ActivationTransactionError::OrphanJournalTemporary);
        }
        if path_exists(&paths.activation_state_temp, "inspect activation temporary")? {
            return Err(ActivationTransactionError::OrphanTemporaryState);
        }

        paths.read_standalone_current()?;

        let journal = ActivationJournal {
            operation,
            before: before.cloned(),
            after: after.clone(),
        };
        let journal_bytes = encode_activation_journal(&journal)?;
        let state_bytes = encode_pointer_state(after)?;

        io.write_new_synced(&paths.activation_journal_temp, &journal_bytes)
            .map_err(|err| io_error("write and sync activation journal temporary", err))?;
        io.rename(&paths.activation_journal_temp, &paths.activation_journal)
            .map_err(|err| io_error("publish activation journal", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync journal directory", err))?;
        io.write_new_synced(&paths.activation_state_temp, &state_bytes)
            .map_err(|err| io_error("write and sync activation state temporary", err))?;
        // Commit the authoritative state only after its package link is durable.
        // Until the rename below, the journal can restore the old selection.
        sync_standalone_current(paths, Some(&after.current), None, io, lock)?;
        io.rename(&paths.activation_state_temp, &paths.activation_state)
            .map_err(|err| io_error("atomically replace activation state", err))?;
        io.sync_dir(&paths.root)
            .map_err(ActivationTransactionError::CommitSync)?;
        io.remove_file(&paths.activation_journal)
            .map_err(|source| ActivationTransactionError::CommittedCleanup {
                operation: "remove activation journal",
                source,
            })?;
        io.sync_dir(&paths.root).map_err(|source| {
            ActivationTransactionError::CommittedCleanup {
                operation: "sync final activation directory",
                source,
            }
        })?;
        Ok(())
    }

    pub(crate) fn recover_activation_state_with_io<I: ActivationIo>(
        paths: &WrapperStatePaths,
        io: &mut I,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        let read_lock = acquire_activation_read_lock(paths)?;
        if read_lock.file.is_none() {
            return Ok(None);
        }
        if !has_pending_activation(paths)? {
            return read_pointer_state(paths);
        }
        drop(read_lock);

        let lock = acquire_activation_lock_with_mode(
            paths,
            FLOCK_EX,
            Some(std::time::Duration::from_secs(2)),
        )?;
        recover_activation_locked(paths, io, &lock).map(|(state, _)| state)
    }

    pub(crate) fn has_pending_activation(
        paths: &WrapperStatePaths,
    ) -> Result<bool, ActivationTransactionError> {
        for path in [
            &paths.activation_journal,
            &paths.activation_journal_temp,
            &paths.activation_state_temp,
        ] {
            if path_exists(path, "inspect pending activation")? {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) fn recover_activation_locked<I: ActivationIo>(
        paths: &WrapperStatePaths,
        io: &mut I,
        lock: &ActivationLockGuard,
    ) -> Result<
        (Option<GenerationPointerState>, Option<ActivationJournal>),
        ActivationTransactionError,
    > {
        let journal_temporary_exists = path_exists(
            &paths.activation_journal_temp,
            "inspect activation journal temporary",
        )?;
        let state_temporary_exists =
            path_exists(&paths.activation_state_temp, "inspect activation temporary")?;
        let journal = read_activation_journal(paths)?;
        let Some(journal) = journal else {
            if journal_temporary_exists {
                if state_temporary_exists {
                    return Err(ActivationTransactionError::RecoveryConflict);
                }
                io.remove_file(&paths.activation_journal_temp)
                    .map_err(|err| io_error("remove stale activation journal temporary", err))?;
                io.sync_dir(&paths.root)
                    .map_err(|err| io_error("sync recovered journal-temporary directory", err))?;
                return Ok((read_pointer_state(paths)?, None));
            }
            if state_temporary_exists {
                return Err(ActivationTransactionError::OrphanTemporaryState);
            }
            return Ok((read_pointer_state(paths)?, None));
        };
        if journal_temporary_exists {
            return Err(ActivationTransactionError::OrphanJournalTemporary);
        }

        if journal.before.as_ref() == Some(&journal.after) {
            return Err(StateFormatError::AmbiguousJournal.into());
        }
        let authoritative = read_pointer_state(paths)?;
        let matches_before = authoritative == journal.before;
        let matches_after = authoritative.as_ref() == Some(&journal.after);
        if matches_before == matches_after {
            return Err(ActivationTransactionError::RecoveryConflict);
        }

        sync_standalone_current(
            paths,
            authoritative.as_ref().map(|state| state.current.as_str()),
            Some(&journal),
            io,
            lock,
        )?;
        if matches_after && paths.standalone_root().is_some() {
            // The writer may have stopped before syncing the state rename.
            io.sync_dir(&paths.root)
                .map_err(ActivationTransactionError::CommitSync)?;
        }
        if state_temporary_exists {
            io.remove_file(&paths.activation_state_temp)
                .map_err(|err| io_error("remove stale activation temporary", err))?;
        }
        io.remove_file(&paths.activation_journal)
            .map_err(|err| io_error("remove recovered activation journal", err))?;
        if let Err(err) = io.sync_dir(&paths.root) {
            if !matches_after || paths.standalone_root().is_none() {
                return Err(io_error("sync recovered activation directory", err));
            }
            // The committed pair is already durable. Losing only the journal
            // deletion on power loss can cause the same recovery to run again.
            eprintln!("codex: warning: activation committed; sync recovered journal deletion failed: {err}");
        }
        Ok((authoritative, matches_after.then_some(journal)))
    }

    pub(crate) fn recover_activation_state(
        paths: &WrapperStatePaths,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        let mut io = FsActivationIo;
        recover_activation_state_with_io(paths, &mut io)
    }
}

#[cfg(test)]
mod activation_interruption_tests {
    use super::generation_state::*;
    use std::fs;
    use std::io;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT_ROOT: AtomicUsize = AtomicUsize::new(0);

    fn isolated_paths() -> WrapperStatePaths {
        let sequence = NEXT_ROOT.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!(
            "wrapper-activation-fault-{}-{sequence}",
            std::process::id()
        ));
        assert!(!root.exists());
        fs::create_dir(&root).unwrap();
        WrapperStatePaths::new(&root).unwrap()
    }

    fn only_expected_files(
        paths: &WrapperStatePaths,
        journal_tmp: bool,
        journal: bool,
        state_tmp: bool,
        state: bool,
    ) {
        assert_eq!(
            paths.activation_journal_temp.exists(),
            journal_tmp,
            "{paths:?}"
        );
        assert_eq!(paths.activation_journal.exists(), journal, "{paths:?}");
        assert_eq!(paths.activation_state_temp.exists(), state_tmp, "{paths:?}");
        assert_eq!(paths.activation_state.exists(), state, "{paths:?}");
    }

    // Real filesystem I/O occurs on every step except the deliberately failed
    // operation; failures are injected immediately before/after each of 8
    // ActivationIo calls and recovery executes through FsActivationIo.
    struct InterruptedIo {
        target: usize,
        after: bool,
        observed: usize,
        disk: FsActivationIo,
    }

    impl InterruptedIo {
        fn operation(
            &mut self,
            action: impl FnOnce(&mut FsActivationIo) -> io::Result<()>,
        ) -> io::Result<()> {
            let index = self.observed;
            self.observed += 1;
            if index == self.target && !self.after {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "fault before I/O",
                ));
            }
            action(&mut self.disk)?;
            if index == self.target {
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "fault after I/O",
                ));
            }
            Ok(())
        }
    }

    impl ActivationIo for InterruptedIo {
        fn write_new_synced(&mut self, path: &Path, data: &[u8]) -> io::Result<()> {
            self.operation(|disk| disk.write_new_synced(path, data))
        }

        fn sync_dir(&mut self, path: &Path) -> io::Result<()> {
            self.operation(|disk| disk.sync_dir(path))
        }

        fn rename(&mut self, from: &Path, to: &Path) -> io::Result<()> {
            self.operation(|disk| disk.rename(from, to))
        }

        fn symlink(&mut self, target: &Path, path: &Path) -> io::Result<()> {
            self.operation(|disk| disk.symlink(target, path))
        }

        fn remove_file(&mut self, path: &Path) -> io::Result<()> {
            self.operation(|disk| disk.remove_file(path))
        }
    }

    fn fixture(
        mode: usize,
    ) -> (
        WrapperStatePaths,
        Option<GenerationPointerState>,
        GenerationPointerState,
    ) {
        let paths = isolated_paths();
        let one = plan_initial_pointer_state("generation-one").unwrap();
        let two = plan_activation_pointer_state(&one, "generation-two").unwrap();
        let (before, after) = match mode {
            0 => (None, one),
            1 => (Some(one), two),
            2 => (
                Some(two.clone()),
                plan_rollback_pointer_state(&two).unwrap(),
            ),
            _ => panic!("unsupported test mode"),
        };
        if let Some(before) = before.as_ref() {
            fs::write(
                &paths.activation_state,
                encode_pointer_state(before).unwrap(),
            )
            .unwrap();
            fs::File::open(&paths.activation_state)
                .unwrap()
                .sync_all()
                .unwrap();
            fs::File::open(&paths.root).unwrap().sync_all().unwrap();
        }
        (paths, before, after)
    }

    #[test]
    fn each_real_io_interruption_recovers_initial_update_and_rollback() {
        for mode in 0..3 {
            for step in 0..8 {
                for after_io in [false, true] {
                    let (paths, before, after) = fixture(mode);
                    let lock = acquire_activation_lock(&paths).unwrap();
                    let mut io = InterruptedIo {
                        target: step,
                        after: after_io,
                        observed: 0,
                        disk: FsActivationIo,
                    };
                    let result = activate_pointer_state_locked(
                        &paths,
                        if mode == 2 {
                            ActivationOperation::Rollback
                        } else {
                            ActivationOperation::Update
                        },
                        before.as_ref(),
                        &after,
                        &mut io,
                        &lock,
                    );
                    assert!(
                        matches!(
                            result,
                            Err(ActivationTransactionError::Io { .. }
                                | ActivationTransactionError::CommitSync(_)
                                | ActivationTransactionError::CommittedCleanup { .. })
                        ),
                        "mode={mode} step={step} after={after_io}: {result:?}"
                    );
                    assert_eq!(io.observed, step + 1);
                    drop(lock);

                    let (journal_tmp, journal, state_tmp) = match (step, after_io) {
                        (0, false) => (false, false, false),
                        (0, true) | (1, false) => (true, false, false),
                        (1, true) | (2, _) | (3, false) => (false, true, false),
                        (3, true) | (4, false) => (false, true, true),
                        (4, true) | (5, _) | (6, false) => (false, true, false),
                        (6, true) | (7, _) => (false, false, false),
                        _ => unreachable!(),
                    };
                    let committed = step > 4 || (step == 4 && after_io);
                    only_expected_files(
                        &paths,
                        journal_tmp,
                        journal,
                        state_tmp,
                        committed || before.is_some(),
                    );

                    let expected = if committed {
                        Some(after.clone())
                    } else {
                        before.clone()
                    };
                    assert_eq!(
                        read_pointer_state(&paths).unwrap(),
                        expected,
                        "before recovery mode={mode} step={step} after={after_io}"
                    );
                    let recovered = recover_activation_state(&paths).unwrap();
                    assert_eq!(
                        recovered, expected,
                        "recover mode={mode} step={step} after={after_io}"
                    );
                    let second = recover_activation_state(&paths).unwrap();
                    assert_eq!(second, expected, "idempotence failed");
                    only_expected_files(&paths, false, false, false, expected.is_some());
                    fs::remove_dir_all(&paths.root).unwrap();
                }
            }
        }
    }

    #[test]
    fn standalone_link_failures_recover_initial_update_and_rollback() {
        for mode in 0..3 {
            // Link creation, rename and sync all precede the state rename.
            for step in 4..7 {
                for after_io in [false, true] {
                    let (paths, before, after) = fixture(mode);
                    let releases = paths.root.join("standalone/releases");
                    fs::create_dir_all(&releases).unwrap();
                    let paths = paths.with_generation_root(&releases);
                    let old_target = before
                        .as_ref()
                        .and_then(|old| paths.standalone_current_target(&old.current));
                    if let Some(old_target) = &old_target {
                        std::os::unix::fs::symlink(
                            old_target,
                            releases.parent().unwrap().join("current"),
                        )
                        .unwrap();
                    }

                    let lock = acquire_activation_lock(&paths).unwrap();
                    let mut fault = InterruptedIo {
                        target: step,
                        after: after_io,
                        observed: 0,
                        disk: FsActivationIo,
                    };
                    let result = activate_pointer_state_locked(
                        &paths,
                        if mode == 2 {
                            ActivationOperation::Rollback
                        } else {
                            ActivationOperation::Update
                        },
                        before.as_ref(),
                        &after,
                        &mut fault,
                        &lock,
                    );
                    assert!(matches!(result, Err(ActivationTransactionError::Io { .. })));
                    assert_eq!(read_pointer_state(&paths).unwrap(), before);
                    assert!(paths.activation_journal.exists());

                    let recovery =
                        recover_activation_locked(&paths, &mut FsActivationIo, &lock).unwrap();
                    assert_eq!(recovery, (before.clone(), None));
                    assert_eq!(paths.read_standalone_current().unwrap(), old_target);
                    assert!(!releases
                        .parent()
                        .unwrap()
                        .join(".current.wrapper.tmp")
                        .is_symlink());

                    activate_pointer_state_locked(
                        &paths,
                        if mode == 2 {
                            ActivationOperation::Rollback
                        } else {
                            ActivationOperation::Update
                        },
                        before.as_ref(),
                        &after,
                        &mut FsActivationIo,
                        &lock,
                    )
                    .unwrap();
                    assert_eq!(read_pointer_state(&paths).unwrap().as_ref(), Some(&after));
                    assert_eq!(
                        paths.read_standalone_current().unwrap(),
                        paths.standalone_current_target(&after.current),
                    );
                    assert!(!has_pending_activation(&paths).unwrap());
                    drop(lock);
                    fs::remove_dir_all(&paths.root).unwrap();
                }
            }
        }
    }

    #[test]
    fn orphan_temporaries_and_conflicts_fail_closed() {
        // An incomplete temporary journal before publication is discardable,
        // but an isolated state temp has no authoritative journal owner.
        for case in 0..4 {
            let (paths, before, after) = fixture(1);
            let journal = ActivationJournal {
                operation: ActivationOperation::Legacy,
                before: before.clone(),
                after,
            };
            let encoded = encode_activation_journal(&journal).unwrap();
            match case {
                0 => fs::write(&paths.activation_journal_temp, &encoded).unwrap(),
                1 => fs::write(&paths.activation_state_temp, b"unowned").unwrap(),
                2 => {
                    fs::write(&paths.activation_journal, &encoded).unwrap();
                    fs::write(&paths.activation_journal_temp, &encoded).unwrap();
                }
                3 => {
                    fs::write(&paths.activation_journal, &encoded).unwrap();
                    let other = plan_initial_pointer_state("other").unwrap();
                    fs::write(
                        &paths.activation_state,
                        encode_pointer_state(&other).unwrap(),
                    )
                    .unwrap();
                }
                _ => unreachable!(),
            };
            let result = recover_activation_state(&paths);
            match case {
                0 => {
                    assert_eq!(result.unwrap(), before);
                    only_expected_files(&paths, false, false, false, true);
                }
                1 => assert!(matches!(
                    result,
                    Err(ActivationTransactionError::OrphanTemporaryState)
                )),
                2 => assert!(matches!(
                    result,
                    Err(ActivationTransactionError::OrphanJournalTemporary)
                )),
                3 => assert!(matches!(
                    result,
                    Err(ActivationTransactionError::RecoveryConflict)
                )),
                _ => unreachable!(),
            }
            if case != 0 {
                assert!(
                    paths.activation_journal_temp.exists()
                        || paths.activation_state_temp.exists()
                        || paths.activation_journal.exists()
                );
            }
            fs::remove_dir_all(&paths.root).unwrap();
        }
    }

    #[test]
    fn recovery_io_interruption_is_repeatable_without_state_loss() {
        // Crash recovery itself can be interrupted at each cleanup I/O.
        // Re-entry must choose the same authoritative before/after pointer.
        for step in 0..3 {
            for after_io in [false, true] {
                let (paths, before, after) = fixture(1);
                let journal = ActivationJournal {
                    operation: ActivationOperation::Legacy,
                    before: before.clone(),
                    after,
                };
                fs::write(
                    &paths.activation_journal,
                    encode_activation_journal(&journal).unwrap(),
                )
                .unwrap();
                fs::write(&paths.activation_state_temp, b"orphaned in-progress state").unwrap();
                let mut io = InterruptedIo {
                    target: step,
                    after: after_io,
                    observed: 0,
                    disk: FsActivationIo,
                };
                let failure = recover_activation_state_with_io(&paths, &mut io);
                assert!(matches!(
                    failure,
                    Err(ActivationTransactionError::Io { .. })
                ));
                assert_eq!(io.observed, step + 1);
                assert_eq!(recover_activation_state(&paths).unwrap(), before);
                assert_eq!(recover_activation_state(&paths).unwrap(), before);
                only_expected_files(&paths, false, false, false, true);
                fs::remove_dir_all(&paths.root).unwrap();
            }
        }
        for step in 0..2 {
            for after_io in [false, true] {
                let (paths, before, _) = fixture(1);
                fs::write(&paths.activation_journal_temp, b"not yet published").unwrap();
                let mut io = InterruptedIo {
                    target: step,
                    after: after_io,
                    observed: 0,
                    disk: FsActivationIo,
                };
                assert!(matches!(
                    recover_activation_state_with_io(&paths, &mut io),
                    Err(ActivationTransactionError::Io { .. })
                ));
                assert_eq!(recover_activation_state(&paths).unwrap(), before);
                only_expected_files(&paths, false, false, false, true);
                fs::remove_dir_all(&paths.root).unwrap();
            }
        }
    }

    #[test]
    fn independent_activation_writer_is_serialized() {
        let (paths, before, after) = fixture(1);
        let first = acquire_activation_lock(&paths).unwrap();
        assert!(matches!(
            acquire_activation_lock(&paths),
            Err(ActivationTransactionError::WriterBusy)
        ));
        // Do not release the lock to mask race errors.
        let mut disk = FsActivationIo;
        activate_pointer_state_locked(
            &paths,
            ActivationOperation::Update,
            before.as_ref(),
            &after,
            &mut disk,
            &first,
        )
        .unwrap();
        drop(first);
        assert_eq!(recover_activation_state(&paths).unwrap(), Some(after));
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn initial_journal_without_pointer_is_recognized_and_cleared() {
        let (paths, _before, after) = fixture(0);
        let journal = ActivationJournal {
            operation: ActivationOperation::Legacy,
            before: None,
            after,
        };
        fs::write(
            &paths.activation_journal,
            encode_activation_journal(&journal).unwrap(),
        )
        .unwrap();
        assert_eq!(recover_activation_state(&paths).unwrap(), None);
        only_expected_files(&paths, false, false, false, false);
        fs::remove_dir_all(&paths.root).unwrap();
    }

    #[test]
    fn clean_readers_share_lock_without_contention() {
        let (paths, _before, after) = fixture(0);
        let writer = acquire_activation_lock(&paths).unwrap();
        let mut disk = FsActivationIo;
        activate_pointer_state_locked(
            &paths,
            ActivationOperation::Update,
            None,
            &after,
            &mut disk,
            &writer,
        )
        .unwrap();
        drop(writer);

        let shared1 = acquire_activation_read_lock(&paths).unwrap();
        let shared2 = acquire_activation_read_lock(&paths).unwrap();
        assert_eq!(recover_activation_state(&paths).unwrap(), Some(after));
        drop(shared2);
        drop(shared1);
        fs::remove_dir_all(&paths.root).unwrap();
    }
}
