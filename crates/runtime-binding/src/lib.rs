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
const CATALOG_MARKER: &[u8] = b"catalog-v1";

fn invalid() -> io::Error {
    io::Error::other("execution binding does not match its namespace")
}

/// Exact legacy namespace; the full binding below rejects hash collisions.
pub fn namespace(home: &Path, program: &OsStr, catalog: Option<&[u8]>) -> String {
    let mut value = 0xcbf29ce484222325_u64;
    for b in home
        .as_os_str()
        .as_bytes()
        .iter()
        .chain([0].iter())
        .chain(program.as_bytes())
    {
        value = (value ^ u64::from(*b)).wrapping_mul(0x100000001b3);
    }
    if let Some(catalog) = catalog {
        for b in b"\0catalog-v1\0".iter().chain(catalog) {
            value = (value ^ u64::from(*b)).wrapping_mul(0x100000001b3);
        }
    }
    format!("{value:016x}")
}

pub fn owner_bytes(home: &Path, program: &OsStr, has_catalog: bool) -> Vec<u8> {
    let mut bytes = home.as_os_str().as_bytes().to_vec();
    bytes.push(0);
    bytes.extend_from_slice(program.as_bytes());
    if has_catalog {
        bytes.push(0);
        bytes.extend_from_slice(CATALOG_MARKER);
    }
    bytes
}

#[derive(Debug)]
pub struct Binding {
    pub home: PathBuf,
    pub program: PathBuf,
}

impl Binding {
    /// The caller supplies its existing owned/private, no-follow bounded reader.
    /// Only bounded owner/catalog bytes are read; PID/peer/liveness stay with the
    /// consumer and must be revalidated before any operation.
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
        let catalog = match fields.next() {
            None => None,
            Some(CATALOG_MARKER) if fields.next().is_none() => {
                let bytes = read(&directory.join("models.json"), CATALOG_LIMIT)?;
                if bytes.is_empty() || bytes.len() as u64 > CATALOG_LIMIT {
                    return Err(invalid());
                }
                Some(bytes)
            }
            _ => return Err(invalid()),
        };
        let family = if catalog.is_some() {
            FAMILIES[1]
        } else {
            FAMILIES[0]
        };
        if directory.parent().and_then(Path::file_name) != Some(OsStr::new(family))
            || directory.file_name()
                != Some(OsStr::new(&namespace(
                    &home,
                    program.as_os_str(),
                    catalog.as_deref(),
                )))
        {
            return Err(invalid());
        }
        Ok(Self { home, program })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode(path: &str, owner: &[u8], catalog: &[u8]) -> io::Result<Binding> {
        Binding::read(Path::new(path), |path, limit| {
            let bytes = if path.file_name() == Some(OsStr::new("owner")) {
                assert_eq!(limit, OWNER_LIMIT);
                owner
            } else {
                assert_eq!(path.file_name(), Some(OsStr::new("models.json")));
                assert_eq!(limit, CATALOG_LIMIT);
                catalog
            };
            Ok(bytes.to_vec())
        })
    }

    #[test]
    fn published_native_and_catalog_bytes_keep_their_namespaces() {
        let home = Path::new("/owned/home");
        let program = OsStr::new("/owned/runtime");
        assert_eq!(namespace(home, program, None), "3b9e6bddaf561c44");
        assert_eq!(
            namespace(home, program, Some(b"opaque-catalog")),
            "0ae7b340bf4873dc"
        );
        for (family, id, catalog) in [
            ("servers", "3b9e6bddaf561c44", None),
            ("catalogs", "0ae7b340bf4873dc", Some(&b"opaque-catalog"[..])),
        ] {
            let owner = owner_bytes(home, program, catalog.is_some());
            let result = decode(
                &format!("/{family}/{id}"),
                &owner,
                catalog.unwrap_or_default(),
            )
            .unwrap();
            assert_eq!(result.home, home);
            assert_eq!(result.program, program);
        }
    }

    #[test]
    fn mismatched_family_owner_catalog_and_unknown_versions_are_rejected() {
        let native = b"/owned/home\0/owned/runtime";
        let custom = b"/owned/home\0/owned/runtime\0catalog-v1";
        for (path, owner, catalog) in [
            ("/catalogs/3b9e6bddaf561c44", &native[..], &b""[..]),
            ("/servers/0ae7b340bf4873dc", custom, b"opaque-catalog"),
            ("/servers/wrong", native, b""),
            (
                "/servers/3b9e6bddaf561c44",
                b"/other/home\0/owned/runtime",
                b"",
            ),
            ("/catalogs/0ae7b340bf4873dc", custom, b"changed"),
            ("/catalogs/0ae7b340bf4873dc", custom, b""),
            (
                "/catalogs/0ae7b340bf4873dc",
                b"/owned/home\0/owned/runtime\0catalog-v2",
                b"opaque-catalog",
            ),
            (
                "/catalogs/0ae7b340bf4873dc",
                b"/owned/home\0/owned/runtime\0catalog-v1\0extra",
                b"opaque-catalog",
            ),
            ("/servers/3b9e6bddaf561c44", b"missing-separator", b""),
        ] {
            assert!(decode(path, owner, catalog).is_err(), "{path}");
        }
        assert!(decode(
            "/servers/3b9e6bddaf561c44",
            &vec![b'x'; OWNER_LIMIT as usize + 1],
            b""
        )
        .is_err());
        assert!(decode(
            "/catalogs/0ae7b340bf4873dc",
            custom,
            &vec![b'x'; CATALOG_LIMIT as usize + 1]
        )
        .is_err());
    }

    #[test]
    fn non_utf8_paths_and_opaque_catalogs_need_no_provider_schema() {
        let home = PathBuf::from(OsString::from_vec(b"/owned/\xff".to_vec()));
        let program = OsString::from_vec(b"/owned/\xfe".to_vec());
        let bytes = b"\xff\x00not-json";
        let directory = Path::new("/catalogs").join(namespace(&home, &program, Some(bytes)));
        let owner = owner_bytes(&home, &program, true);
        let binding = Binding::read(&directory, |path, _| {
            Ok(if path.file_name() == Some(OsStr::new("owner")) {
                owner.clone()
            } else {
                bytes.to_vec()
            })
        })
        .unwrap();
        assert_eq!(binding.home, home);
        assert_eq!(binding.program.as_os_str(), program);
        assert!(Binding::read(&directory, |_, _| Err(io::Error::from(
            io::ErrorKind::PermissionDenied
        )))
        .is_err());
    }
}
