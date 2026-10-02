//! Release-specific adaptation of the upstream protected UDS rendezvous root.
pub(super) const POLICY: &str = "termux-fd-remap-v2";
pub(super) const RAW_SHA256: &str =
    "50b06603bdcdac39b714f5c3e68583c002b8ad8779ebfdaaf4932ff016b379c0";
const PATCHES: &[(usize, &[u8], &[u8])] = &[
    (
        0x9db8a74,
        &[
            0xe8, 0x85, 0x8e, 0x52, 0xe0, 0x63, 0x00, 0x91, 0xa8, 0x0d, 0xae, 0x72, 0xa1, 0x00,
            0x80, 0x52, 0xff, 0x73, 0x00, 0x39, 0xe8, 0x1b, 0x00, 0xb9,
        ],
        &[
            0x40, 0xd5, 0x00, 0xb0, 0x00, 0x80, 0x2a, 0x91, 0x1f, 0x20, 0x03, 0xd5, 0x21, 0x02,
            0x80, 0x52, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5,
        ],
    ),
    (
        0x9db8b50,
        &[0xa0, 0xd8, 0x00, 0x90, 0x00, 0x10, 0x0b, 0x91],
        &[0x40, 0xd5, 0x00, 0xb0, 0x00, 0x00, 0x2b, 0x91],
    ),
    (0xb861aa0, &[0; 17], b"/proc/self/fd/35\0"),
    (0xb861ac0, &[0; 2], b"\xc0\0"),
];

pub(super) fn required(version: &str) -> Result<bool, &'static str> {
    let parts = version
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "unqualified upstream UDS version")?;
    if parts.len() != 3 || parts.as_slice() > [0, 160, 0].as_slice() {
        return Err("upstream UDS version requires fresh Termux qualification");
    }
    Ok(parts == [0, 160, 0])
}

pub(super) fn apply(bytes: &mut [u8], raw_digest: &str) -> Result<usize, &'static str> {
    if raw_digest != RAW_SHA256
        || bytes.get(0xb8cc2c4..0xb8cc2d4) != Some(b"\x0dcodex-daemon-\xc0\0")
        || PATCHES.iter().any(|(off, old, new)| {
            old.len() != new.len() || bytes.get(*off..off + old.len()) != Some(*old)
        })
    {
        return Err("upstream 0.160.0 UDS artifact does not match qualified policy");
    }
    let changed = PATCHES
        .iter()
        .map(|(_, old, new)| old.iter().zip(*new).filter(|(a, b)| a != b).count())
        .sum();
    for (off, _, new) in PATCHES {
        bytes[*off..off + new.len()].copy_from_slice(new);
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uds_artifact_patch_is_exact_bounded_and_rejects_drift() {
        assert_eq!(required("0.159.2"), Ok(false));
        assert_eq!(required("0.160.0"), Ok(true));
        for future in ["0.160.1", "0.161.0", "1.0.0", "invalid"] {
            assert!(required(future).is_err());
        }
        let mut bytes = vec![0; 0xb8cc2d4];
        bytes[0xb8cc2c4..0xb8cc2d4].copy_from_slice(b"\x0dcodex-daemon-\xc0\0");
        for (off, old, _) in PATCHES {
            bytes[*off..off + old.len()].copy_from_slice(old);
        }
        assert!(apply(&mut bytes, "unqualified").is_err());
        bytes[0x9db8a74] ^= 1;
        assert!(apply(&mut bytes, RAW_SHA256).is_err());
        bytes[0x9db8a74] ^= 1;
        let before = bytes.clone();
        assert_eq!(apply(&mut bytes, RAW_SHA256).unwrap(), 43);
        assert_eq!(before.len(), bytes.len());
        for (index, (a, b)) in before.iter().zip(&bytes).enumerate() {
            if a != b {
                assert!(PATCHES
                    .iter()
                    .any(|(off, old, _)| (*off..off + old.len()).contains(&index)));
            }
        }
        assert!(apply(&mut bytes, RAW_SHA256).is_err());
    }
}
