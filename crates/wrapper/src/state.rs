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
pub(super) mod m2_generation_state {
    use super::ReleasePublicKey;
    use crate::{flock, FLOCK_EX, FLOCK_NB, FLOCK_UN};
    use std::io::{Read, Write};
    use std::os::fd::AsRawFd;

    const GENERATION_ID_MAX_BYTES: usize = 512;
    const STATE_FILE_MAX_BYTES: usize = 16 * 1024;
    const STATE_FORMAT: &str = "codex-activation-state-v3";
    const JOURNAL_FORMAT: &str = "codex-activation-journal-v3";

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub(crate) struct CoreStatePaths {
        pub(crate) root: std::path::PathBuf,
        pub(crate) activation_state: std::path::PathBuf,
        pub(crate) activation_journal: std::path::PathBuf,
        pub(crate) activation_journal_temp: std::path::PathBuf,
        pub(crate) activation_state_temp: std::path::PathBuf,
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
        pub(crate) before: Option<GenerationPointerState>,
        pub(crate) after: GenerationPointerState,
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
                ActivationTransactionError::Io { source, .. } => Some(source),
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

    pub(crate) fn acquire_activation_lock(
        paths: &CoreStatePaths,
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
        let result = unsafe { flock(file.as_raw_fd(), FLOCK_EX | FLOCK_NB) };
        if result == 0 {
            return Ok(ActivationLockGuard { file: Some(file) });
        }
        let source = std::io::Error::last_os_error();
        if source.kind() == std::io::ErrorKind::WouldBlock {
            return Err(ActivationTransactionError::WriterBusy);
        }
        Err(io_error("acquire wrapper state writer lock", source))
    }

    impl CoreStatePaths {
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
            })
        }
    }

    pub(crate) fn prepare_core_state_paths(
        paths: &CoreStatePaths,
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
        expected_records: usize,
    ) -> Result<Vec<&'a str>, StateFormatError> {
        if bytes.len() > STATE_FILE_MAX_BYTES {
            return Err(StateFormatError::FileTooLarge(label));
        }
        let text = std::str::from_utf8(bytes).map_err(|_| StateFormatError::InvalidUtf8(label))?;
        if !text.ends_with('\n') {
            return Err(StateFormatError::MissingFinalNewline(label));
        }
        let body = &text[..text.len() - 1];
        let records: Vec<_> = body.split('\n').collect();
        if records.len() != expected_records {
            return Err(StateFormatError::InvalidRecordCount(label));
        }
        Ok(records)
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
        let records = parse_lines(bytes, "activation state", 7)?;
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
        Ok(format!(
            "format={JOURNAL_FORMAT}\nbefore_present={before_present}\nbefore_update_key={before_update_key}\nbefore_current={before_current}\nbefore_current_key={before_current_key}\nbefore_previous_present={before_previous_present}\nbefore_previous={before_previous}\nbefore_previous_key={before_previous_key}\nafter_update_key={}\nafter_current={}\nafter_current_key={}\nafter_previous_present={after_previous_present}\nafter_previous={after_previous}\nafter_previous_key={after_previous_key}\n",
            journal.after.update_key.to_hex(),
            journal.after.current,
            journal.after.current_key.to_hex(),
        )
        .into_bytes())
    }

    pub(crate) fn parse_activation_journal(
        bytes: &[u8],
    ) -> Result<ActivationJournal, StateFormatError> {
        let records = parse_lines(bytes, "activation journal", 14)?;
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
        Ok(ActivationJournal { before, after })
    }

    pub(crate) trait ActivationIo {
        fn write_new_synced(&mut self, path: &std::path::Path, data: &[u8]) -> std::io::Result<()>;
        fn sync_dir(&mut self, path: &std::path::Path) -> std::io::Result<()>;
        fn rename(&mut self, from: &std::path::Path, to: &std::path::Path) -> std::io::Result<()>;
        fn remove_file(&mut self, path: &std::path::Path) -> std::io::Result<()>;
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
        paths: &CoreStatePaths,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        read_bounded_regular_file(&paths.activation_state, "activation state")?
            .map(|bytes| parse_pointer_state(&bytes).map_err(ActivationTransactionError::from))
            .transpose()
    }

    fn read_activation_journal(
        paths: &CoreStatePaths,
    ) -> Result<Option<ActivationJournal>, ActivationTransactionError> {
        read_bounded_regular_file(&paths.activation_journal, "activation journal")?
            .map(|bytes| parse_activation_journal(&bytes).map_err(ActivationTransactionError::from))
            .transpose()
    }

    pub(crate) fn activate_pointer_state_locked<I: ActivationIo>(
        paths: &CoreStatePaths,
        before: Option<&GenerationPointerState>,
        after: &GenerationPointerState,
        io: &mut I,
        _lock: &ActivationLockGuard,
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

        let journal = ActivationJournal {
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
        io.rename(&paths.activation_state_temp, &paths.activation_state)
            .map_err(|err| io_error("atomically replace activation state", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync activation state directory", err))?;
        io.remove_file(&paths.activation_journal)
            .map_err(|err| io_error("remove activation journal", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync final activation directory", err))?;
        Ok(())
    }

    pub(crate) fn recover_activation_state_with_io<I: ActivationIo>(
        paths: &CoreStatePaths,
        io: &mut I,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        let _lock = acquire_activation_lock(paths)?;
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
                return read_pointer_state(paths);
            }
            if state_temporary_exists {
                return Err(ActivationTransactionError::OrphanTemporaryState);
            }
            return read_pointer_state(paths);
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

        if state_temporary_exists {
            io.remove_file(&paths.activation_state_temp)
                .map_err(|err| io_error("remove stale activation temporary", err))?;
        }
        io.remove_file(&paths.activation_journal)
            .map_err(|err| io_error("remove recovered activation journal", err))?;
        io.sync_dir(&paths.root)
            .map_err(|err| io_error("sync recovered activation directory", err))?;
        Ok(authoritative)
    }

    pub(crate) fn recover_activation_state(
        paths: &CoreStatePaths,
    ) -> Result<Option<GenerationPointerState>, ActivationTransactionError> {
        let mut io = FsActivationIo;
        recover_activation_state_with_io(paths, &mut io)
    }
}
