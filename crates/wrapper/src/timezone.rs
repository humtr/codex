//! Supply Android's timezone data to the Linux musl runtime and its frontend.
use std::ffi::{OsStr, OsString};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::{MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::process::Command;

// Android's active module precedes the legacy updater and system image.
const DATABASES: [&str; 3] = [
    "/apex/com.android.tzdata/etc/tz/tzdata",
    "/data/misc/zoneinfo/current/tzdata",
    "/system/usr/share/zoneinfo/tzdata",
];
// Bound allocations at external file boundaries, not timezone/transition counts.
const DATABASE_BYTE_LIMIT: usize = 16 * 1024 * 1024;
const ZONE_BYTE_LIMIT: usize = 1024 * 1024;

pub(super) fn configure(command: &mut Command) {
    configure_with_cache(command, None);
}

pub(super) fn configure_with_cache(command: &mut Command, cache_base: Option<&Path>) {
    if !Path::new("/system/bin/getprop").is_file() {
        return;
    }
    let explicit = command
        .get_envs()
        .find(|(name, _)| *name == "TZ")
        .map(|(_, value)| value.map(OsStr::to_owned))
        .unwrap_or_else(|| std::env::var_os("TZ"));
    let result: io::Result<Option<OsString>> = (|| {
        let Some(zone) = selected_zone(explicit.as_deref(), || {
            let output = Command::new("/system/bin/getprop")
                .arg("persist.sys.timezone")
                .output()?;
            if !output.status.success() {
                return Err(invalid("Android timezone property query failed"));
            }
            String::from_utf8(output.stdout)
                .map(|value| value.trim().to_owned())
                .map_err(|_| invalid("Android timezone property is not UTF-8"))
        })?
        else {
            return Ok(None);
        };
        let base = cache_base
            .map(Path::to_path_buf)
            .or_else(|| {
                std::env::var_os("XDG_CACHE_HOME")
                    .filter(|value| !value.is_empty())
                    .map(PathBuf::from)
            })
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".cache")))
            .filter(|path| path.is_absolute())
            .ok_or_else(|| invalid("timezone cache needs an absolute XDG_CACHE_HOME or HOME"))?;
        let parent = base.join("codex");
        fs::create_dir_all(&parent)?;
        let cache = parent.canonicalize()?.join("timezones");
        let mut database = open_database(&DATABASES.map(PathBuf::from))?;
        let stamp = database_stamp(&database.metadata()?);
        load_cached_zone(&cache, &zone, &stamp, || {
            let bytes = read_bounded(&mut database, DATABASE_BYTE_LIMIT)?;
            if database_stamp(&database.metadata()?) != stamp {
                return Err(invalid("Android tzdata changed during extraction"));
            }
            Ok(bytes)
        })
        .map(Some)
    })();
    match result {
        Ok(Some(tz)) => {
            command.env("TZ", tz);
        }
        Ok(None) => (),
        Err(error) => {
            eprintln!("codex: Android timezone normalization failed: {error}; leaving TZ unchanged")
        }
    }
}

fn invalid(reason: &'static str) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, reason)
}

fn valid_zone_name(zone: &str) -> bool {
    !zone.is_empty()
        && zone.len() < 40 // Android index names occupy a NUL-terminated 40-byte field.
        && zone.split('/').all(|part| {
            !part.is_empty()
                && part != "."
                && part != ".."
                && part.bytes().all(|c| c.is_ascii_alphanumeric() || b"_+-".contains(&c))
        })
}

fn selected_zone(
    explicit: Option<&OsStr>,
    system_zone: impl FnOnce() -> io::Result<String>,
) -> io::Result<Option<String>> {
    let zone = match explicit {
        Some(value) => {
            use std::os::unix::ffi::OsStrExt;
            let bytes = value.as_bytes();
            let explicit_name = bytes.starts_with(b":");
            let value = OsStr::from_bytes(bytes.strip_prefix(b":").unwrap_or(bytes));
            if Path::new(value).is_absolute() {
                return Ok(None);
            }
            let value = value.to_str().ok_or_else(|| invalid("TZ is not UTF-8"))?;
            // Empty TZ means UTC. File paths and POSIX rules already work in musl.
            if value.is_empty()
                || value.starts_with('.')
                || value.contains(',')
                || (!explicit_name
                    && !value.contains('/')
                    && value.bytes().any(|c| c.is_ascii_digit()))
            {
                return Ok(None);
            }
            value.to_owned()
        }
        None => system_zone()?,
    };
    if !valid_zone_name(&zone) {
        return Err(invalid("invalid Android/IANA timezone name"));
    }
    Ok(Some(zone))
}

fn read_bounded(file: impl Read, limit: usize) -> io::Result<Vec<u8>> {
    let mut bytes = Vec::new();
    file.take(limit as u64 + 1).read_to_end(&mut bytes)?;
    if bytes.len() > limit {
        return Err(invalid("timezone data exceeds its byte limit"));
    }
    Ok(bytes)
}

fn open_database(paths: &[PathBuf]) -> io::Result<File> {
    for path in paths {
        match OpenOptions::new()
            .read(true)
            .custom_flags(0o4000)
            .open(path)
        {
            Ok(file) if file.metadata()?.is_file() => return Ok(file),
            Ok(_) => return Err(invalid("Android tzdata is not a regular file")),
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                match fs::symlink_metadata(path) {
                    Err(missing) if missing.kind() == io::ErrorKind::NotFound => (),
                    _ => return Err(error),
                }
            }
            Err(error) => {
                return Err(io::Error::new(
                    error.kind(),
                    format!("{}: {error}", path.display()),
                ))
            }
        }
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        "Android tzdata is unavailable",
    ))
}

#[cfg(test)]
fn read_database(paths: &[PathBuf]) -> io::Result<Vec<u8>> {
    read_bounded(open_database(paths)?, DATABASE_BYTE_LIMIT)
}

fn database_stamp(meta: &fs::Metadata) -> String {
    format!(
        "v1 {} {} {} {} {}\n",
        meta.dev(),
        meta.ino(),
        meta.len(),
        meta.mtime(),
        meta.mtime_nsec(),
    )
}

fn number(bytes: &[u8], offset: usize) -> io::Result<usize> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("truncated timezone header"))?;
    Ok(u32::from_be_bytes(value.try_into().unwrap()) as usize)
}

fn extract_zone<'a>(database: &'a [u8], zone: &str) -> io::Result<&'a [u8]> {
    if !valid_zone_name(zone)
        || database.get(..6) != Some(b"tzdata")
        || database.get(11) != Some(&0)
        || !database
            .get(6..10)
            .is_some_and(|v| v.iter().all(u8::is_ascii_digit))
        || !database.get(10).is_some_and(u8::is_ascii_lowercase)
    {
        return Err(invalid("invalid Android tzdata header or zone name"));
    }
    let index = number(database, 12)?;
    let data = number(database, 16)?;
    let end = number(database, 20)?;
    if index < 24 || index >= data || data > end || end > database.len() || (data - index) % 52 != 0
    {
        return Err(invalid("invalid Android tzdata index bounds"));
    }
    for entry in database[index..data].as_chunks::<52>().0 {
        let nul = entry[..40]
            .iter()
            .position(|c| *c == 0)
            .ok_or_else(|| invalid("unterminated Android timezone name"))?;
        if &entry[..nul] != zone.as_bytes() {
            continue;
        }
        let offset = number(entry, 40)?;
        let length = number(entry, 44)?;
        if offset > end - data || length > end - data - offset || length > ZONE_BYTE_LIMIT {
            return Err(invalid(
                "Android timezone payload is outside the data section",
            ));
        }
        let bytes = &database[data + offset..data + offset + length];
        validate_tzif(bytes)?;
        return Ok(bytes);
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Android timezone not found: {zone}"),
    ))
}

fn tzif_block(bytes: &[u8], start: usize, width: usize) -> io::Result<usize> {
    let header = bytes
        .get(start..start + 44)
        .ok_or_else(|| invalid("truncated TZif header"))?;
    if &header[..4] != b"TZif" {
        return Err(invalid("invalid TZif magic"));
    }
    let utc = number(header, 20)?;
    let standard = number(header, 24)?;
    let leaps = number(header, 28)?;
    let times = number(header, 32)?;
    let types = number(header, 36)?;
    let chars = number(header, 40)?;
    if types == 0
        || types > 256
        || chars == 0
        || (utc != 0 && utc != types)
        || (standard != 0 && standard != types)
    {
        return Err(invalid("invalid TZif counts"));
    }
    // Counts are u32, and supported Termux hosts are 64-bit; use checked arithmetic
    // as this parser is also built and tested on other Unix hosts.
    let length = times
        .checked_mul(width + 1)
        .and_then(|n| n.checked_add(types * 6))
        .and_then(|n| n.checked_add(chars))
        .and_then(|n| leaps.checked_mul(width + 4).and_then(|m| n.checked_add(m)))
        .and_then(|n| n.checked_add(standard))
        .and_then(|n| n.checked_add(utc))
        .ok_or_else(|| invalid("TZif size overflow"))?;
    let body_start = start + 44;
    let end = body_start
        .checked_add(length)
        .filter(|end| *end <= bytes.len())
        .ok_or_else(|| invalid("truncated TZif data"))?;
    let body = &bytes[body_start..end];
    let indices = times * width;
    let mut previous = None;
    for transition in body[..indices].chunks_exact(width) {
        let time = if width == 4 {
            i64::from(i32::from_be_bytes(transition.try_into().unwrap()))
        } else {
            i64::from_be_bytes(transition.try_into().unwrap())
        };
        if previous.is_some_and(|old| time <= old) {
            return Err(invalid("unordered TZif transitions"));
        }
        previous = Some(time);
    }
    if body[indices..indices + times]
        .iter()
        .any(|v| usize::from(*v) >= types)
    {
        return Err(invalid("invalid TZif transition type"));
    }
    let info = indices + times;
    let abbreviations = &body[info + types * 6..info + types * 6 + chars];
    for entry in body[info..info + types * 6].as_chunks::<6>().0 {
        if entry[4] > 1
            || !abbreviations
                .get(usize::from(entry[5])..)
                .is_some_and(|v| v.contains(&0))
        {
            return Err(invalid("invalid TZif local time type"));
        }
    }
    if body[body.len() - standard - utc..].iter().any(|v| *v > 1) {
        return Err(invalid("invalid TZif standard/UTC indicator"));
    }
    Ok(end)
}

fn validate_tzif(bytes: &[u8]) -> io::Result<()> {
    let first_end = tzif_block(bytes, 0, 4)?;
    let end = match bytes[4] {
        0 => first_end,
        version @ (b'2' | b'3' | b'4') => {
            let end = tzif_block(bytes, first_end, 8)?;
            if bytes[first_end + 4] != version {
                return Err(invalid("inconsistent TZif versions"));
            }
            let footer = &bytes[end..];
            if footer.len() < 2
                || footer[0] != b'\n'
                || footer.last() != Some(&b'\n')
                || !footer[1..footer.len() - 1]
                    .iter()
                    .all(|c| c.is_ascii_graphic())
            {
                return Err(invalid("invalid TZif POSIX footer"));
            }
            bytes.len()
        }
        _ => return Err(invalid("unsupported TZif version")),
    };
    if end != bytes.len() {
        return Err(invalid("unexpected bytes after TZif data"));
    }
    Ok(())
}

fn zone_path(cache: &Path, zone: &str) -> PathBuf {
    // '/' is the only valid zone-name byte escaped here; '%' cannot occur in a name.
    cache.join(format!("{}.tzif", zone.replace('/', "%")))
}

fn read_cache_file(path: &Path, limit: u64) -> io::Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(meta)
            if meta.is_file()
                && meta.uid() == super::fs_util::current_uid()?
                && meta.permissions().mode() & 0o777 == 0o600 =>
        {
            super::fs_util::read_bounded_regular(path, limit, true, "invalid timezone cache file")
                .map(Some)
        }
        Ok(_) => Err(invalid("timezone cache file is not a private regular file")),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(err) => Err(err),
    }
}

fn write_cache_file(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let cache = path
        .parent()
        .ok_or_else(|| invalid("timezone cache has no parent"))?;
    let temporary = cache.join(format!(
        ".zone-{}-{}",
        std::process::id(),
        crate::fs_util::LOCAL_STAGING_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)?;
    let result = (|| {
        if file.metadata()?.permissions().mode() & 0o777 != 0o600 {
            return Err(invalid("timezone output file is not private and readable"));
        }
        file.write_all(bytes)?;
        file.sync_all()?;
        fs::rename(&temporary, path)?;
        File::open(cache)?.sync_all()
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

fn tz_value(path: &Path) -> OsString {
    let mut tz = OsString::from(":");
    tz.push(path);
    tz
}

fn load_cached_zone(
    cache: &Path,
    zone: &str,
    stamp: &str,
    read_database: impl FnOnce() -> io::Result<Vec<u8>>,
) -> io::Result<OsString> {
    super::fs_util::private_dir(cache)?;
    let path = zone_path(cache, zone);
    let stamp_path = path.with_extension("stamp");
    let cached = read_cache_file(&path, ZONE_BYTE_LIMIT as u64)?;
    let cached_stamp = read_cache_file(&stamp_path, 256)?;
    // Bind the stamp to the exact TZif bytes so readers never accept a mixed
    // cache pair when another process is publishing or is interrupted.
    let stamped_zone =
        |bytes: &[u8]| format!("{stamp}{}", codex_release_builder::sha256_bytes(bytes));
    if let Some(bytes) = cached.as_deref() {
        if cached_stamp.as_deref() == Some(stamped_zone(bytes).as_bytes())
            && validate_tzif(bytes).is_ok()
        {
            return Ok(tz_value(&path));
        }
    }

    let database = read_database()?;
    let bytes = extract_zone(&database, zone)?;
    if cached.as_deref() != Some(bytes) {
        write_cache_file(&path, bytes)?;
    }
    write_cache_file(&stamp_path, stamped_zone(bytes).as_bytes())?;
    Ok(tz_value(&path))
}

#[cfg(test)]
fn publish_zone(cache: &Path, zone: &str, bytes: &[u8]) -> io::Result<OsString> {
    let data = tests::database(zone, bytes);
    let stamp = codex_release_builder::sha256_bytes(&data);
    load_cached_zone(cache, zone, &stamp, || Ok(data))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixed_zone(offset: i32) -> Vec<u8> {
        let mut bytes = vec![0; 44];
        bytes[..4].copy_from_slice(b"TZif");
        bytes[36..40].copy_from_slice(&1u32.to_be_bytes());
        bytes[40..44].copy_from_slice(&4u32.to_be_bytes());
        bytes.extend(offset.to_be_bytes());
        bytes.extend([0, 0]);
        bytes.extend(b"TST\0");
        bytes
    }

    pub(super) fn database(zone: &str, payload: &[u8]) -> Vec<u8> {
        let mut bytes = b"tzdata2026c\0".to_vec();
        for offset in [24, 76, 76 + payload.len() as u32] {
            bytes.extend(offset.to_be_bytes());
        }
        let mut entry = [0; 52];
        entry[..zone.len()].copy_from_slice(zone.as_bytes());
        entry[44..48].copy_from_slice(&(payload.len() as u32).to_be_bytes());
        bytes.extend(entry);
        bytes.extend(payload);
        bytes
    }

    #[test]
    fn explicit_rules_paths_and_empty_tz_do_not_query_android() {
        for value in [
            "",
            "KST-9",
            "UTC0",
            "<+09>-9",
            "EST5EDT,M3.2.0/2,M11.1.0/2",
            "/private/zone.tzif",
            ":/private/zone.tzif",
            ":./zone.tzif",
        ] {
            assert_eq!(
                selected_zone(Some(OsStr::new(value)), || panic!(
                    "unwanted property query"
                ))
                .unwrap(),
                None
            );
        }
        for value in ["Asia/Seoul", ":America/New_York", "Etc/GMT-9", "UTC"] {
            assert_eq!(
                selected_zone(Some(OsStr::new(value)), || panic!(
                    "unwanted property query"
                ))
                .unwrap()
                .as_deref(),
                Some(value.trim_start_matches(':'))
            );
        }
        assert_eq!(
            selected_zone(None, || Ok("Asia/Seoul".into()))
                .unwrap()
                .as_deref(),
            Some("Asia/Seoul")
        );
        for value in [
            "",
            "Asia/../Seoul",
            "Asia//Seoul",
            "Asia/Seoul\n",
            "/Asia/Seoul",
        ] {
            assert!(selected_zone(None, || Ok(value.into())).is_err());
        }
        assert!(selected_zone(None, || Err(invalid("property unavailable"))).is_err());
    }

    #[test]
    fn android_index_and_tzif_faults_are_rejected() {
        let good = database("Asia/Seoul", &fixed_zone(9 * 3600));
        assert_eq!(
            extract_zone(&good, "Asia/Seoul").unwrap(),
            fixed_zone(9 * 3600)
        );
        assert!(extract_zone(&good, "America/New_York").is_err());
        for end in 0..good.len() {
            assert!(
                extract_zone(&good[..end], "Asia/Seoul").is_err(),
                "truncation at {end}"
            );
        }
        for (offset, bytes) in [
            (12, 0u32.to_be_bytes()),
            (16, 25u32.to_be_bytes()),
            (20, u32::MAX.to_be_bytes()),
            (64, u32::MAX.to_be_bytes()),
            (68, u32::MAX.to_be_bytes()),
            (76, *b"bad!"),
        ] {
            let mut bad = good.clone();
            bad[offset..offset + 4].copy_from_slice(&bytes);
            assert!(
                extract_zone(&bad, "Asia/Seoul").is_err(),
                "fault at {offset}"
            );
        }
        for (offset, value) in [(4, b'9'), (39, 0), (43, 0), (48, 2), (49, 4), (53, b'!')] {
            let mut bad = fixed_zone(9 * 3600);
            bad[offset] = value;
            assert!(validate_tzif(&bad).is_err(), "TZif fault at {offset}");
        }
        // A v2 payload includes both time widths and a delimited POSIX footer.
        let mut v2 = fixed_zone(9 * 3600);
        v2[4] = b'2';
        v2.extend(v2.clone());
        v2.extend(b"\nTST-9\n");
        validate_tzif(&v2).unwrap();
        v2.pop();
        assert!(validate_tzif(&v2).is_err());
    }

    #[test]
    fn colon_prefixed_numeric_iana_names_select_database_entries() {
        for zone in ["EST5EDT", "GMT0", "CST6CDT", "MST7MDT", "PST8PDT"] {
            let explicit = format!(":{zone}");
            assert_eq!(
                selected_zone(Some(OsStr::new(&explicit)), || panic!(
                    "unexpected system query"
                ))
                .unwrap()
                .as_deref(),
                Some(zone)
            );
            assert_eq!(
                selected_zone(Some(OsStr::new(zone)), || panic!("unexpected system query"))
                    .unwrap(),
                None
            );
        }
    }

    #[test]
    fn unchanged_database_stamp_and_tzif_skip_database_reads() {
        let root = crate::tests::temp_root("timezone-cache-hit");
        let source = root.join("active");
        let cache = root.join("cache");
        let zone = "EST5EDT";
        let first = fixed_zone(-5 * 3600);
        fs::write(&source, database(zone, &first)).unwrap();
        let metadata = || database_stamp(&fs::metadata(&source).unwrap());
        let stamp = metadata();
        load_cached_zone(&cache, zone, &stamp, || fs::read(&source)).unwrap();
        let path = zone_path(&cache, zone);
        let inode = fs::metadata(&path).unwrap().ino();
        assert_eq!(
            load_cached_zone(&cache, zone, &stamp, || panic!(
                "cache hit read Android tzdata"
            ))
            .unwrap(),
            tz_value(&path)
        );
        assert_eq!(fs::metadata(&path).unwrap().ino(), inode);

        let second = fixed_zone(-4 * 3600);
        let mut changed = database(zone, &second);
        changed.extend_from_slice(b"extra trailing database bytes");
        fs::write(&source, &changed).unwrap();
        let fresh = metadata();
        assert_ne!(fresh, stamp);
        load_cached_zone(&cache, zone, &fresh, || fs::read(&source)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), second);
        // A valid but mismatched cached TZif must not be accepted under an unchanged stamp.
        fs::write(&path, first).unwrap();
        load_cached_zone(&cache, zone, &fresh, || fs::read(&source)).unwrap();
        assert_eq!(fs::read(&path).unwrap(), second);
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn active_android_source_and_changed_zone_bytes_are_published() {
        let root = crate::tests::temp_root("timezone-refresh");
        let sources = [
            root.join("active"),
            root.join("legacy"),
            root.join("system"),
        ];
        let cache = root.join("timezones");
        let old = fixed_zone(0);
        let current = fixed_zone(9 * 3600);
        fs::write(&sources[2], database("Asia/Seoul", &old)).unwrap();
        fs::write(&sources[0], database("Asia/Seoul", &current)).unwrap();
        let bytes = read_database(&sources).unwrap();
        let tz = publish_zone(
            &cache,
            "Asia/Seoul",
            extract_zone(&bytes, "Asia/Seoul").unwrap(),
        )
        .unwrap();
        let path = Path::new(tz.to_str().unwrap().strip_prefix(':').unwrap());
        assert_eq!(fs::read(path).unwrap(), current);
        let inode = fs::metadata(path).unwrap().ino();
        assert_eq!(publish_zone(&cache, "Asia/Seoul", &current).unwrap(), tz);
        assert_eq!(fs::metadata(path).unwrap().ino(), inode);

        // Same database version, changed payload: refresh actual bytes, not a version stamp.
        let updated = fixed_zone(10 * 3600);
        fs::write(&sources[0], database("Asia/Seoul", &updated)).unwrap();
        let bytes = read_database(&sources).unwrap();
        publish_zone(
            &cache,
            "Asia/Seoul",
            extract_zone(&bytes, "Asia/Seoul").unwrap(),
        )
        .unwrap();
        assert_eq!(fs::read(path).unwrap(), updated);
        // A corrupt present active database must not silently select an older one.
        fs::write(&sources[0], b"corrupt").unwrap();
        assert!(extract_zone(&read_database(&sources).unwrap(), "Asia/Seoul").is_err());
        fs::remove_file(&sources[0]).unwrap();
        std::os::unix::fs::symlink(root.join("missing-database"), &sources[0]).unwrap();
        assert!(read_database(&sources).is_err());
        fs::remove_file(&sources[0]).unwrap();
        assert_eq!(
            extract_zone(&read_database(&sources).unwrap(), "Asia/Seoul").unwrap(),
            old
        );
        crate::tests::remove_temp_root(root);
    }

    #[test]
    fn cache_rejects_symlinks_and_non_private_files() {
        let root = crate::tests::temp_root("timezone-private");
        let cache = root.join("timezones");
        let zone = fixed_zone(9 * 3600);
        publish_zone(&cache, "Asia/Seoul", &zone).unwrap();
        let path = cache.join("Asia%Seoul.tzif");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();
        assert!(publish_zone(&cache, "Asia/Seoul", &zone).is_err());
        fs::remove_file(&path).unwrap();
        let foreign = root.join("foreign");
        fs::write(&foreign, b"preserve").unwrap();
        std::os::unix::fs::symlink(&foreign, &path).unwrap();
        assert!(publish_zone(&cache, "Asia/Seoul", &zone).is_err());
        assert_eq!(fs::read(&foreign).unwrap(), b"preserve");
        crate::tests::remove_temp_root(root);
    }

    #[cfg(target_os = "android")]
    #[test]
    fn common_launch_boundary_delivers_android_tzif_to_child() {
        const CHILD: &str = "CODEX_TEST_TIMEZONE_CHILD";
        if let Ok(zone) = std::env::var(CHILD) {
            let mut command = Command::new("/system/bin/sh");
            command.args(["-c", "printf '%s' \"$TZ\""]);
            crate::runtime_env::apply_child_env_plan_and_fence(
                &mut command,
                Some(&crate::runtime_env::TermuxBaseEnvPlan {
                    assignments: vec![],
                    removals: vec![],
                }),
            );
            configure(&mut command);
            let output = command.output().unwrap();
            assert!(output.status.success());
            let tz = String::from_utf8(output.stdout).unwrap();
            if zone == "KST-9" || zone.is_empty() || zone == "Invalid/Timezone" {
                assert_eq!(tz, zone);
            } else {
                let path = Path::new(
                    tz.strip_prefix(':')
                        .expect("wrapper must select a TZif path"),
                );
                assert!(path.starts_with(std::env::var_os("XDG_CACHE_HOME").unwrap()));
                let data = read_database(&DATABASES.map(PathBuf::from)).unwrap();
                assert_eq!(fs::read(path).unwrap(), extract_zone(&data, &zone).unwrap());
            }
            return;
        }
        let root = crate::tests::temp_root("timezone-android-child");
        let system = Command::new("/system/bin/getprop")
            .arg("persist.sys.timezone")
            .output()
            .unwrap();
        assert!(system.status.success());
        let system = String::from_utf8(system.stdout).unwrap();
        for (explicit, zone) in [
            (Some("Asia/Seoul"), "Asia/Seoul"),
            (Some("America/New_York"), "America/New_York"),
            (Some("KST-9"), "KST-9"),
            (Some(""), ""),
            (Some("Invalid/Timezone"), "Invalid/Timezone"),
            (None, system.trim()),
        ] {
            let mut child = Command::new(std::env::current_exe().unwrap());
            child
                .args([
                    "--exact",
                    "timezone::tests::common_launch_boundary_delivers_android_tzif_to_child",
                    "--nocapture",
                ])
                .env(CHILD, zone)
                .env_remove("TZ")
                .env("XDG_CACHE_HOME", &root);
            if let Some(value) = explicit {
                child.env("TZ", value);
            }
            let output = child.output().unwrap();
            let stdout = String::from_utf8_lossy(&output.stdout);
            assert!(
                output.status.success() && stdout.contains("1 passed"),
                "{stdout}\n{}",
                String::from_utf8_lossy(&output.stderr)
            );
            if zone == "Invalid/Timezone" {
                assert!(String::from_utf8_lossy(&output.stderr)
                    .contains("Android timezone not found: Invalid/Timezone"));
            }
        }
        crate::tests::remove_temp_root(root);
    }
}
