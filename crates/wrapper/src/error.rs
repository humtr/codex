use crate::doctor;
use crate::generation::{GenerationManifestError, RuntimeAssetError};
use crate::runtime_env::TermuxProcessEnvError;
use crate::state::generation_state;
use std::io;

#[derive(Debug)]
pub(super) enum LocalProductError {
    MissingEnvironment(&'static str),
    InvalidEnvironmentPath(&'static str),
    StateFormat(generation_state::StateFormatError),
    State(generation_state::ActivationTransactionError),
    NoCurrentGeneration,
    Io {
        operation: &'static str,
        source: io::Error,
    },
    Descriptor(&'static str),
    UnsafeSource(&'static str),
    GenerationCollision,
    CurlUnavailable,
    Remote(&'static str),
    RemoteTransportFailed {
        code: Option<i32>,
        http_status: Option<u16>,
        bytes: u64,
        elapsed_seconds: u64,
    },
    RemoteStalled {
        bytes: u64,
        idle_seconds: u64,
    },
    RemoteResponseTooLarge,
    LocalUpdate(&'static str),
    Builder(codex_release_builder::BuilderError),
    ReleaseDigestMismatch,
    CandidateProbe(&'static str),
    Manifest(GenerationManifestError),
    Runtime(RuntimeAssetError),
    Dispatch(PublicDispatchExecutionError),
}

impl std::fmt::Display for LocalProductError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocalProductError::MissingEnvironment(name) => {
                write!(f, "required environment variable {name} is missing")
            }
            LocalProductError::InvalidEnvironmentPath(name) => {
                write!(f, "environment variable {name} must be a normalized absolute path")
            }
            LocalProductError::StateFormat(err) => err.fmt(f),
            LocalProductError::State(err) => err.fmt(f),
            LocalProductError::NoCurrentGeneration => f.write_str(
                "no Codex generation is currently installed; run 'codex update' to install an upstream release",
            ),
            LocalProductError::Io { operation, source } => write!(f, "{operation}: {source}"),
            LocalProductError::Descriptor(msg) => f.write_str(msg),
            LocalProductError::UnsafeSource(msg) => f.write_str(msg),
            LocalProductError::GenerationCollision => {
                f.write_str("target generation directory already exists")
            }
            LocalProductError::CurlUnavailable => {
                f.write_str("required 'curl' binary is unavailable in Termux PREFIX")
            }
            LocalProductError::Remote(msg) => f.write_str(msg),
            LocalProductError::RemoteTransportFailed {
                code,
                http_status,
                bytes,
                elapsed_seconds,
            } => {
                write!(
                    f,
                    "download failed (exit={}, http={}, bytes={}, elapsed={}s)",
                    code.map_or_else(|| "-".to_owned(), |c| c.to_string()),
                    http_status.map_or_else(|| "-".to_owned(), |s| s.to_string()),
                    bytes,
                    elapsed_seconds
                )?;
                if matches!(http_status, Some(403 | 429)) {
                    f.write_str(". If you hit the GitHub API rate limit, try 'codex update --version <X.Y.Z>' with an exact release version")?;
                }
                Ok(())
            }
            LocalProductError::RemoteStalled { bytes, idle_seconds } => write!(
                f,
                "download stalled after {bytes} bytes ({idle_seconds}s without progress)"
            ),
            LocalProductError::RemoteResponseTooLarge => {
                f.write_str("remote response exceeded allowed size bound")
            }
            LocalProductError::LocalUpdate(msg) => f.write_str(msg),
            LocalProductError::Builder(err) => err.fmt(f),
            LocalProductError::ReleaseDigestMismatch => {
                f.write_str("downloaded release archive SHA-256 digest mismatch")
            }
            LocalProductError::CandidateProbe(msg) => f.write_str(msg),
            LocalProductError::Manifest(err) => err.fmt(f),
            LocalProductError::Runtime(err) => err.fmt(f),
            LocalProductError::Dispatch(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for LocalProductError {}

#[derive(Debug)]
pub(super) enum RuntimeLaunchError {
    Environment(TermuxProcessEnvError),
    Config(io::Error),
    Exec(io::Error),
}

impl std::fmt::Display for RuntimeLaunchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeLaunchError::Environment(err) => err.fmt(f),
            RuntimeLaunchError::Config(err) => {
                write!(f, "failed to prepare Termux config: {err}")
            }
            RuntimeLaunchError::Exec(err) => {
                write!(f, "failed to exec upstream runtime: {err}")
            }
        }
    }
}

impl std::error::Error for RuntimeLaunchError {}

#[derive(Debug)]
pub(super) enum PublicDispatchExecutionError {
    Upstream(RuntimeLaunchError),
    Doctor(doctor::LocalDoctorCommandError),
}

impl std::fmt::Display for PublicDispatchExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublicDispatchExecutionError::Upstream(err) => err.fmt(f),
            PublicDispatchExecutionError::Doctor(err) => err.fmt(f),
        }
    }
}

impl std::error::Error for PublicDispatchExecutionError {}
