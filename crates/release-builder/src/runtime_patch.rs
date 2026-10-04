//! Exact-artifact Termux adaptation of the protected UDS root and approval UI.
pub(super) const POLICY: &str = "termux-fd-remap-v3";
pub(super) const PREVIOUS_POLICY: &str = "termux-fd-remap-v2";
pub(super) const PERMISSION_POLICY: &str = "termux-permission-picker-0-160-0-v1";
pub(super) const CHANGED_BYTES: usize = 430;
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
    // builtin_approval_presets: no-sandbox auto profile, allocation/copy/drop layout.
    (0x9dbfc08, &[0x40, 0x01, 0x80, 0x52], &[0x60, 0x02, 0x80, 0x52]),
    (0x9dbfc24, &[0x29, 0x99, 0x0b, 0x91, 0x68, 0xac, 0x8c, 0x52, 0x29, 0x01, 0x40, 0xf9, 0xe0, 0x03, 0xc0, 0x3d, 0x08, 0x10, 0x00, 0x79, 0x48, 0x01, 0x80, 0x52, 0x09, 0x00, 0x00, 0xf9], &[0x29, 0x6d, 0x0e, 0x91, 0x28, 0xf1, 0x40, 0xb8, 0x20, 0x01, 0xc0, 0x3d, 0x08, 0xf0, 0x00, 0xb8, 0x00, 0x00, 0x80, 0x3d, 0x68, 0x02, 0x80, 0x52, 0xe0, 0x03, 0xc0, 0x3d]),
    (0x9dbfdbc, &[0x41, 0x01, 0x80, 0x52], &[0x61, 0x02, 0x80, 0x52]),
    // Named permissions_menu: Ask/Auto point to the existing full-access ID.
    (0x95de89c, &[0x84, 0x14, 0x2e, 0x91], &[0x84, 0x08, 0x2f, 0x91]),
    (0x95de948, &[0x84, 0x14, 0x2e, 0x91], &[0x84, 0x08, 0x2f, 0x91]),
    (0x95de8b4, &[0x45, 0x01, 0x80, 0x52], &[0x65, 0x02, 0x80, 0x52]),
    (0x95de960, &[0x45, 0x01, 0x80, 0x52], &[0x65, 0x02, 0x80, 0x52]),
    // UI descriptions; preserve their original byte lengths.
    (0xbd69ac3, b"Read and edit workspace files and run commands, with approval required for internet access or edits outside the workspace", b"Run without a sandbox; the selected reviewer handles approval requests required by command rules and policy.             "),
    (0x9dbfc48, &[0xa8, 0xa3, 0x00, 0xd1, 0x00, 0x01, 0x80, 0x52, 0xe1, 0x03, 0x1f, 0xaa, 0xe2, 0x03, 0x1f, 0x2a, 0xe3, 0x03, 0x1f, 0x2a, 0xe4, 0x03, 0x1f, 0x2a, 0xa9, 0x4c, 0x95, 0x97], &[0xa8, 0x0a, 0x00, 0xf9, 0xe9, 0x07, 0x41, 0xb2, 0xe9, 0xb7, 0x00, 0xf9, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5]),
    (0xbda72f7, b"Codex can read and edit files in the current workspace, and run commands. Approval is required to access the internet or edit other files. (Identical to Agent mode)", b"Codex runs without a sandbox. Approval requests follow command rules and policy; the selected reviewer handles those requests.                                      "),
    (0xbd69b8f, b"Only ask for actions detected as potentially unsafe", b"Auto-review approval requests without a sandbox.   "),
    // preset_matches_current: auto matches Disabled, preserving approval comparison.
    (0x95cf99c, &[0x80, 0x04, 0x00, 0x54], &[0x20, 0xfc, 0xff, 0x54]),
    // Permission shortcuts omit Read Only; Full Access remains menu-only upstream.
    (0x956ee00, &[0x01, 0xf9, 0xff, 0x54], &[0xc8, 0xff, 0xff, 0x17]),
    // Named popup omits Read Only and carries the existing vector length.
    (0x95dec14, &[0xc8, 0x02, 0x18, 0x8b, 0x08, 0xa5, 0x45, 0xa9], &[0xf6, 0x03, 0x08, 0xaa, 0x8f, 0x00, 0x00, 0x14]),
    // Legacy popup copies the final description word inline: load the new tail.
    (0x95d288c, &[0x68, 0x2e, 0x8c, 0x52, 0xc8, 0xac, 0xac, 0x72], &[0x28, 0xf1, 0x42, 0xb8, 0x1f, 0x20, 0x03, 0xd5]),
    // Do not increment that vector length for the omitted Read Only entry.
    (0x95dee64, &[0xc9, 0x06, 0x00, 0x91], &[0xe9, 0x03, 0x16, 0xaa]),
    // Named popup has a second inline tail copy of AUTO_REVIEW_DESCRIPTION.
    (0x95de918, &[0x69, 0x2e, 0x8c, 0x52], &[0x49, 0xf1, 0x42, 0xb8]),
    (0x95de920, &[0xc9, 0xac, 0xac, 0x72], &[0x1f, 0x20, 0x03, 0xd5]),

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
        return Err("upstream 0.160.0 artifact does not match qualified Termux policy");
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
    fn runtime_artifact_patch_is_exact_bounded_and_rejects_drift() {
        assert_eq!(required("0.159.2"), Ok(false));
        assert_eq!(required("0.160.0"), Ok(true));
        for future in ["0.160.1", "0.161.0", "1.0.0", "invalid"] {
            assert!(required(future).is_err());
        }
        let mut bytes = vec![0; 0xbda72f7 + 164];
        bytes[0xb8cc2c4..0xb8cc2d4].copy_from_slice(b"\x0dcodex-daemon-\xc0\0");
        for (off, old, _) in PATCHES {
            bytes[*off..off + old.len()].copy_from_slice(old);
        }
        assert!(apply(&mut bytes, "unqualified").is_err());
        // Every qualified block is a fail-closed boundary, before any mutation.
        for (off, old, _) in PATCHES {
            bytes[*off] ^= 1;
            assert!(apply(&mut bytes, RAW_SHA256).is_err(), "drift at {off:x}");
            bytes[*off] ^= 1;
            for (check, expected, _) in PATCHES {
                assert_eq!(&bytes[*check..check + expected.len()], *expected);
            }
            assert!(apply(&mut bytes[..off + old.len() - 1], RAW_SHA256).is_err());
        }
        let mut ranges = PATCHES
            .iter()
            .map(|(off, old, _)| *off..off + old.len())
            .collect::<Vec<_>>();
        ranges.sort_by_key(|range| range.start);
        assert!(ranges.windows(2).all(|pair| pair[0].end <= pair[1].start));
        let before = bytes.clone();
        assert_eq!(apply(&mut bytes, RAW_SHA256).unwrap(), CHANGED_BYTES);
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
