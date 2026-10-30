//! Core's existing execution-record contract, shared with optional consumers.
//! No model schemas, provider policy, filesystem mutation or process lifecycle.
#![cfg(unix)]

use std::ffi::{OsStr, OsString};
use std::io;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

pub const FAMILIES: [&str; 2] = ["servers", "catalogs"];
pub const OWNER_LIMIT: u64 = 16384;
pub const CATALOG_LIMIT: u64 = 4 * 1024 * 1024;
pub const CONFIG_LIMIT: u64 = 16384;
pub const INVOCATION_LIMIT: u64 = CONFIG_LIMIT;
const CATALOG_MARKER: &[u8] = b"catalog-v1";
const CONFIG_MARKER: &[u8] = b"config-v1";
const INVOCATION_MARKER: &[u8] = b"invocation-v1";

fn invalid() -> io::Error {
    io::Error::other("execution binding does not match its namespace")
}

/// Versioned opaque settings binding. This short hash routes records; producers
/// must also compare their complete requested snapshot before reusing a server.
pub fn configuration_namespace(
    home: &Path,
    program: &OsStr,
    catalog: Option<&[u8]>,
    invocation: Option<&[u8]>,
    config: &[u8],
    requirements: Option<&[u8]>,
) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    let mut field = |bytes: Option<&[u8]>| {
        let length = bytes.map_or(u64::MAX, |b| b.len() as u64).to_le_bytes();
        for byte in length.iter().chain(bytes.unwrap_or_default()) {
            hash = (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3);
        }
    };
    field(Some(CONFIG_MARKER));
    field(Some(home.as_os_str().as_bytes()));
    field(Some(program.as_bytes()));
    field(catalog);
    field(Some(config));
    field(requirements);
    if let Some(invocation) = invocation {
        field(Some(INVOCATION_MARKER));
        field(Some(invocation));
    }
    format!("{hash:016x}")
}

pub fn configuration_owner_bytes(
    home: &Path,
    program: &OsStr,
    has_catalog: bool,
    has_invocation: bool,
) -> Vec<u8> {
    let mut bytes = home.as_os_str().as_bytes().to_vec();
    bytes.push(0);
    bytes.extend_from_slice(program.as_bytes());
    bytes.push(0);
    bytes.extend_from_slice(CONFIG_MARKER);
    if has_catalog {
        bytes.push(0);
        bytes.extend_from_slice(CATALOG_MARKER);
    }
    if has_invocation {
        bytes.push(0);
        bytes.extend_from_slice(INVOCATION_MARKER);
    }
    bytes
}

/// Only standard configuration arguments can precede the fixed app-server command.
/// Their values remain opaque; upstream owns TOML/profile interpretation and policy.
pub fn encode_invocation(args: &[OsString]) -> io::Result<Option<Vec<u8>>> {
    if args.is_empty() {
        return Ok(None);
    }
    let size = validate_invocation(args)?;
    let mut bytes = Vec::with_capacity(size);
    for arg in args {
        bytes.extend_from_slice(arg.as_bytes());
        bytes.push(0);
    }
    Ok(Some(bytes))
}

fn invalid_invocation() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "invalid shared server configuration arguments",
    )
}

fn validate_invocation(args: &[OsString]) -> io::Result<usize> {
    let (pairs, remainder) = args.as_chunks::<2>();
    if !remainder.is_empty() {
        return Err(invalid_invocation());
    }
    let mut size = 0;
    for pair in pairs {
        if !matches!(pair[0].to_str(), Some("--config" | "--profile")) || pair[1].is_empty() {
            return Err(invalid_invocation());
        }
        for arg in pair {
            let value = arg.to_str().ok_or_else(invalid_invocation)?.as_bytes();
            size += value.len() + 1;
            if value.contains(&0) || size as u64 > INVOCATION_LIMIT {
                return Err(invalid_invocation());
            }
        }
    }
    Ok(size)
}

pub fn decode_invocation(bytes: &[u8]) -> io::Result<Vec<OsString>> {
    if bytes.is_empty() || bytes.len() as u64 > INVOCATION_LIMIT {
        return Err(invalid_invocation());
    }
    let values = bytes.strip_suffix(&[0]).ok_or_else(invalid_invocation)?;
    let args = values
        .split(|b| *b == 0)
        .map(|v| {
            std::str::from_utf8(v)
                .map(OsString::from)
                .map_err(|_| invalid_invocation())
        })
        .collect::<io::Result<Vec<_>>>()?;
    validate_invocation(&args)?;
    Ok(args)
}

pub fn settings_identity(config: &[u8], requirements: Option<&[u8]>) -> String {
    configuration_namespace(
        Path::new("/"),
        OsStr::new("core-system-config-v1"),
        None,
        None,
        config,
        requirements,
    )
}

#[derive(Debug)]
pub struct Binding {
    pub home: PathBuf,
    pub program: PathBuf,
}

impl Binding {
    /// The caller supplies its existing owned/private, no-follow bounded reader.
    /// Only bounded owner/catalog/settings bytes are read; PID/peer/liveness stay
    /// with the consumer. Readers must preserve NotFound for an absent optional
    /// requirements file, and reject unsafe paths before returning any bytes.
    pub fn read(
        directory: &Path,
        mut read: impl FnMut(&Path, u64) -> io::Result<Vec<u8>>,
    ) -> io::Result<Self> {
        let owner = read(&directory.join("owner"), OWNER_LIMIT)?;
        if owner.len() as u64 > OWNER_LIMIT {
            return Err(invalid());
        }
        let mut fields = owner.split(|b| *b == 0);
        let home = PathBuf::from(OsString::from_vec(
            fields.next().ok_or_else(invalid)?.to_vec(),
        ));
        let program = PathBuf::from(OsString::from_vec(
            fields.next().ok_or_else(invalid)?.to_vec(),
        ));
        let tail: Vec<_> = fields.collect();
        let (has_catalog, has_invocation) = match tail.as_slice() {
            [CONFIG_MARKER] => (false, false),
            [CONFIG_MARKER, CATALOG_MARKER] => (true, false),
            [CONFIG_MARKER, INVOCATION_MARKER] => (false, true),
            [CONFIG_MARKER, CATALOG_MARKER, INVOCATION_MARKER] => (true, true),
            _ if tail.first().copied() != Some(CONFIG_MARKER) => {
                return Err(io::Error::new(
                    io::ErrorKind::Unsupported,
                    "unsupported execution-record format",
                ))
            }
            _ => return Err(invalid()),
        };
        let invocation = if has_invocation {
            let bytes = read(&directory.join("invocation"), INVOCATION_LIMIT)?;
            decode_invocation(&bytes)?;
            Some(bytes)
        } else {
            None
        };
        let catalog = if has_catalog {
            let bytes = read(&directory.join("models.json"), CATALOG_LIMIT)?;
            if bytes.is_empty() || bytes.len() as u64 > CATALOG_LIMIT {
                return Err(invalid());
            }
            Some(bytes)
        } else {
            None
        };
        let config = read(&directory.join("config/config.toml"), CONFIG_LIMIT)?;
        if config.len() as u64 > CONFIG_LIMIT {
            return Err(invalid());
        }
        let requirements = match read(&directory.join("config/requirements.toml"), CONFIG_LIMIT) {
            Ok(bytes) if bytes.len() as u64 <= CONFIG_LIMIT => Some(bytes),
            Err(e) if e.kind() == io::ErrorKind::NotFound => None,
            _ => return Err(invalid()),
        };
        let expected = configuration_namespace(
            &home,
            program.as_os_str(),
            catalog.as_deref(),
            invocation.as_deref(),
            &config,
            requirements.as_deref(),
        );
        let family = if catalog.is_some() || invocation.is_some() {
            FAMILIES[1]
        } else {
            FAMILIES[0]
        };
        if directory.parent().and_then(Path::file_name) != Some(OsStr::new(family))
            || directory.file_name() != Some(OsStr::new(&expected))
        {
            return Err(invalid());
        }
        Ok(Self { home, program })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_records_bind_opaque_settings_presence_and_both_families() {
        let home = Path::new("/owned/home");
        let program = OsStr::new("/owned/runtime");
        let config = b"opaque\0config\xff";
        for catalog in [None, Some(&b"opaque catalog"[..])] {
            for requirements in [None, Some(&b""[..]), Some(&b"opaque requirements"[..])] {
                let id =
                    configuration_namespace(home, program, catalog, None, config, requirements);
                let dir = Path::new(if catalog.is_some() {
                    "/catalogs"
                } else {
                    "/servers"
                })
                .join(&id);
                let owner = configuration_owner_bytes(home, program, catalog.is_some(), false);
                let read = |path: &Path, limit| {
                    let bytes = match path.strip_prefix(&dir).unwrap().to_str().unwrap() {
                        "owner" => {
                            assert_eq!(limit, OWNER_LIMIT);
                            Some(owner.as_slice())
                        }
                        "models.json" => {
                            assert_eq!(limit, CATALOG_LIMIT);
                            catalog
                        }
                        "config/config.toml" => {
                            assert_eq!(limit, CONFIG_LIMIT);
                            Some(&config[..])
                        }
                        "config/requirements.toml" => {
                            assert_eq!(limit, CONFIG_LIMIT);
                            requirements
                        }
                        other => panic!("unexpected read: {other}"),
                    };
                    bytes
                        .map(Vec::from)
                        .ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
                };
                let binding = Binding::read(&dir, read).unwrap();
                assert_eq!(binding.home, home);
                assert_eq!(binding.program, Path::new(program));
                // A same-home/settings change selects a different immutable record.
                assert_ne!(
                    id,
                    configuration_namespace(home, program, catalog, None, b"changed", requirements)
                );

                // No absence/empty-file or concatenation ambiguity.
                assert_ne!(
                    configuration_namespace(home, program, catalog, None, config, None),
                    configuration_namespace(home, program, catalog, None, config, Some(b""))
                );
                assert_ne!(
                    configuration_namespace(home, program, catalog, None, b"a", Some(b"bc")),
                    configuration_namespace(home, program, catalog, None, b"ab", Some(b"c"))
                );
                assert!(Binding::read(&dir, |p, limit| {
                    if p.ends_with("config/config.toml") {
                        Ok(b"substituted".to_vec())
                    } else {
                        read(p, limit)
                    }
                })
                .is_err());
                assert!(Binding::read(&dir, |p, limit| {
                    if p.ends_with("config/requirements.toml") {
                        Err(io::Error::from(io::ErrorKind::PermissionDenied))
                    } else {
                        read(p, limit)
                    }
                })
                .is_err());
                assert!(Binding::read(&dir, |p, limit| {
                    if p.ends_with("config/config.toml") {
                        Ok(vec![b'x'; CONFIG_LIMIT as usize + 1])
                    } else {
                        read(p, limit)
                    }
                })
                .is_err());
            }
        }
    }

    #[test]
    fn unknown_versions_do_not_authorize_operations_and_current_records_are_strict() {
        let home = Path::new("/owned/home");
        let program = OsStr::new("/owned/runtime");
        let config = b"settings";
        let id = configuration_namespace(home, program, None, None, config, None);
        let dir = Path::new("/servers").join(id);
        for owner in [
            b"/owned/home\0/owned/runtime".to_vec(),
            b"/owned/home\0/owned/runtime\0config-v9".to_vec(),
            b"/owned/home\0/owned/runtime\0config-v1\0unexpected".to_vec(),
            vec![b'x'; OWNER_LIMIT as usize + 1],
        ] {
            assert!(Binding::read(&dir, |path, _| if path.ends_with("owner") {
                Ok(owner.clone())
            } else {
                panic!("rejected owner must not read settings")
            })
            .is_err());
        }
        let owner = configuration_owner_bytes(home, program, false, false);
        assert!(Binding::read(
            &Path::new("/catalogs").join(dir.file_name().unwrap()),
            |path, _| {
                if path.ends_with("owner") {
                    Ok(owner.clone())
                } else if path.ends_with("requirements.toml") {
                    Err(io::Error::from(io::ErrorKind::NotFound))
                } else {
                    Ok(config.to_vec())
                }
            }
        )
        .is_err());
    }

    #[test]
    fn invocation_records_bind_arguments_and_reject_non_configuration_execution() {
        let args =
            ["--profile", "work", "--config", "model_provider=\"local\""].map(OsString::from);
        let bytes = encode_invocation(&args).unwrap().unwrap();
        assert_eq!(
            bytes,
            b"--profile\0work\0--config\0model_provider=\"local\"\0"
        );
        assert_eq!(decode_invocation(&bytes).unwrap(), args);
        assert!(encode_invocation(&[]).unwrap().is_none());
        for bad in [
            &b""[..],
            b"--profile\0work",
            b"--profile\0\0",
            b"exec\0effect\0",
            b"--config\0x=1\0app-server\0",
            b"--profile\0\xff\0",
        ] {
            assert!(decode_invocation(bad).is_err());
        }
        assert!(encode_invocation(&[
            "--config".into(),
            "x".repeat(INVOCATION_LIMIT as usize).into()
        ])
        .is_err());
        let home = Path::new("/owned/home");
        let runtime = OsStr::new("/owned/runtime");
        for catalog in [None, Some(&b"catalog"[..])] {
            let id = configuration_namespace(home, runtime, catalog, Some(&bytes), b"config", None);
            let plain = configuration_namespace(home, runtime, catalog, None, b"config", None);
            assert_ne!(id, plain);
            let directory = Path::new("/catalogs").join(id);
            let owner = configuration_owner_bytes(home, runtime, catalog.is_some(), true);
            let read = |path: &Path, _: u64| match path
                .strip_prefix(&directory)
                .unwrap()
                .to_str()
                .unwrap()
            {
                "owner" => Ok(owner.clone()),
                "invocation" => Ok(bytes.clone()),
                "models.json" => Ok(catalog.unwrap().to_vec()),
                "config/config.toml" => Ok(b"config".to_vec()),
                "config/requirements.toml" => Err(io::ErrorKind::NotFound.into()),
                other => panic!("unexpected field {other}"),
            };
            Binding::read(&directory, read).unwrap();
            assert!(Binding::read(&directory, |path, limit| {
                if path.ends_with("invocation") {
                    Ok(b"--profile\0other\0".to_vec())
                } else {
                    read(path, limit)
                }
            })
            .is_err());
        }
    }
}
