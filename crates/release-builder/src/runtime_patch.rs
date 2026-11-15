//! Exact-artifact Termux adaptation of the protected UDS root and approval UI.
pub(super) const POLICY: &str = "termux-fd-remap-v3";
pub(super) const PREVIOUS_POLICY: &str = "termux-fd-remap-v2";
pub(super) const PERMISSION_POLICY: &str = "termux-permission-picker-0-160-0-v1";
pub(super) const CHANGED_BYTES: usize = 430;
pub(super) const RAW_SHA256: &str =
    "50b06603bdcdac39b714f5c3e68583c002b8ad8779ebfdaaf4932ff016b379c0";
type Patch = (usize, &'static [u8], &'static [u8]);
const PATCHES: &[Patch] = &[
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

const PATCHES_161: &[Patch] = &[
    // Shortcut omits Read Only.
    (
        0x9f09db8,
        &[0x01, 0xf9, 0xff, 0x54],
        &[0xc8, 0xff, 0xff, 0x17],
    ),
    // Legacy inline Auto description tail.
    (
        0x9f6b588,
        &[0x68, 0x2e, 0x8c, 0x52, 0xc8, 0xac, 0xac, 0x72],
        &[0x28, 0xf1, 0x42, 0xb8, 0x1f, 0x20, 0x03, 0xd5],
    ),
    // Auto current marker matches Disabled.
    (
        0x9f6c238,
        &[0x80, 0x04, 0x00, 0x54],
        &[0x20, 0xfc, 0xff, 0x54],
    ),
    // Named full-access ID pointer.
    (
        0x9f7b074,
        &[0x84, 0x38, 0x1b, 0x91],
        &[0x84, 0x2c, 0x1c, 0x91],
    ),
    // Named full-access ID length.
    (
        0x9f7b08c,
        &[0x45, 0x01, 0x80, 0x52],
        &[0x65, 0x02, 0x80, 0x52],
    ),
    // Named inline Auto description tail load.
    (
        0x9f7b0f0,
        &[0x69, 0x2e, 0x8c, 0x52],
        &[0x49, 0xf1, 0x42, 0xb8],
    ),
    // Named inline Auto description tail immediate removed.
    (
        0x9f7b0f8,
        &[0xc9, 0xac, 0xac, 0x72],
        &[0x1f, 0x20, 0x03, 0xd5],
    ),
    // Named full-access ID pointer.
    (
        0x9f7b120,
        &[0x84, 0x38, 0x1b, 0x91],
        &[0x84, 0x2c, 0x1c, 0x91],
    ),
    // Named full-access ID length.
    (
        0x9f7b138,
        &[0x45, 0x01, 0x80, 0x52],
        &[0x65, 0x02, 0x80, 0x52],
    ),
    // Named popup skips Read Only and preserves shared scratch pointer.
    (
        0x9f7b3a8,
        &[
            0x68, 0xa7, 0x45, 0xa9, 0x3f, 0x11, 0x00, 0xf1, 0x40, 0x04, 0x00, 0x54,
        ],
        &[
            0xf5, 0x03, 0x08, 0xaa, 0xf9, 0x03, 0x1e, 0x91, 0x96, 0x00, 0x00, 0x14,
        ],
    ),
    // Named popup retains vector length.
    (
        0x9f7b618,
        &[0xa9, 0x06, 0x00, 0x91],
        &[0xe9, 0x03, 0x15, 0xaa],
    ),
    // UDS canonical FD35 path.
    (
        0xa0b45a0,
        &[
            0xe8, 0x85, 0x8e, 0x52, 0xe0, 0x63, 0x00, 0x91, 0xa8, 0x0d, 0xae, 0x72, 0xa1, 0x00,
            0x80, 0x52, 0xff, 0x73, 0x00, 0x39, 0xe8, 0x1b, 0x00, 0xb9,
        ],
        &[
            0x40, 0xd5, 0x00, 0xd0, 0x00, 0x80, 0x2a, 0x91, 0x1f, 0x20, 0x03, 0xd5, 0x21, 0x02,
            0x80, 0x52, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5,
        ],
    ),
    // UDS UID-only formatting.
    (
        0xa0b467c,
        &[0xa0, 0xd8, 0x00, 0xb0, 0x00, 0x3c, 0x2b, 0x91],
        &[0x40, 0xd5, 0x00, 0xd0, 0x00, 0x00, 0x2b, 0x91],
    ),
    // Auto ID allocation.
    (
        0xa0bb734,
        &[0x40, 0x01, 0x80, 0x52],
        &[0x60, 0x02, 0x80, 0x52],
    ),
    // Auto full-access ID copy and lengths.
    (
        0xa0bb750,
        &[
            0x29, 0x69, 0x15, 0x91, 0x68, 0xac, 0x8c, 0x52, 0x29, 0x01, 0x40, 0xf9, 0xe0, 0x03,
            0xc0, 0x3d, 0x08, 0x10, 0x00, 0x79, 0x48, 0x01, 0x80, 0x52, 0x09, 0x00, 0x00, 0xf9,
        ],
        &[
            0x29, 0x3d, 0x18, 0x91, 0x28, 0xf1, 0x40, 0xb8, 0x20, 0x01, 0xc0, 0x3d, 0x08, 0xf0,
            0x00, 0xb8, 0x00, 0x00, 0x80, 0x3d, 0x68, 0x02, 0x80, 0x52, 0xe0, 0x03, 0xc0, 0x3d,
        ],
    ),
    // Auto Disabled discriminant.
    (
        0xa0bb774,
        &[
            0xa8, 0xa3, 0x00, 0xd1, 0x00, 0x01, 0x80, 0x52, 0xe1, 0x03, 0x1f, 0xaa, 0xe2, 0x03,
            0x1f, 0x2a, 0xe3, 0x03, 0x1f, 0x2a, 0xe4, 0x03, 0x1f, 0x2a, 0x3f, 0x33, 0x90, 0x97,
        ],
        &[
            0xa8, 0x0a, 0x00, 0xf9, 0xe9, 0x07, 0x41, 0xb2, 0xe9, 0xb7, 0x00, 0xf9, 0x1f, 0x20,
            0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5, 0x1f, 0x20, 0x03, 0xd5,
        ],
    ),
    // Auto OOM layout.
    (
        0xa0bb8e8,
        &[0x41, 0x01, 0x80, 0x52],
        &[0x61, 0x02, 0x80, 0x52],
    ),
    // UDS padding path.
    (
        0xbb5eaa0,
        &[
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00,
        ],
        &[
            0x2f, 0x70, 0x72, 0x6f, 0x63, 0x2f, 0x73, 0x65, 0x6c, 0x66, 0x2f, 0x66, 0x64, 0x2f,
            0x33, 0x35, 0x00,
        ],
    ),
    // UDS padding UID format.
    (0xbb5eac0, &[0x00, 0x00], &[0xc0, 0x00]),
    // Permission description.
    (
        0xc0a760c,
        &[
            0x52, 0x65, 0x61, 0x64, 0x20, 0x61, 0x6e, 0x64, 0x20, 0x65, 0x64, 0x69, 0x74, 0x20,
            0x77, 0x6f, 0x72, 0x6b, 0x73, 0x70, 0x61, 0x63, 0x65, 0x20, 0x66, 0x69, 0x6c, 0x65,
            0x73, 0x20, 0x61, 0x6e, 0x64, 0x20, 0x72, 0x75, 0x6e, 0x20, 0x63, 0x6f, 0x6d, 0x6d,
            0x61, 0x6e, 0x64, 0x73, 0x2c, 0x20, 0x77, 0x69, 0x74, 0x68, 0x20, 0x61, 0x70, 0x70,
            0x72, 0x6f, 0x76, 0x61, 0x6c, 0x20, 0x72, 0x65, 0x71, 0x75, 0x69, 0x72, 0x65, 0x64,
            0x20, 0x66, 0x6f, 0x72, 0x20, 0x69, 0x6e, 0x74, 0x65, 0x72, 0x6e, 0x65, 0x74, 0x20,
            0x61, 0x63, 0x63, 0x65, 0x73, 0x73, 0x20, 0x6f, 0x72, 0x20, 0x65, 0x64, 0x69, 0x74,
            0x73, 0x20, 0x6f, 0x75, 0x74, 0x73, 0x69, 0x64, 0x65, 0x20, 0x74, 0x68, 0x65, 0x20,
            0x77, 0x6f, 0x72, 0x6b, 0x73, 0x70, 0x61, 0x63, 0x65,
        ],
        &[
            0x52, 0x75, 0x6e, 0x20, 0x77, 0x69, 0x74, 0x68, 0x6f, 0x75, 0x74, 0x20, 0x61, 0x20,
            0x73, 0x61, 0x6e, 0x64, 0x62, 0x6f, 0x78, 0x3b, 0x20, 0x74, 0x68, 0x65, 0x20, 0x73,
            0x65, 0x6c, 0x65, 0x63, 0x74, 0x65, 0x64, 0x20, 0x72, 0x65, 0x76, 0x69, 0x65, 0x77,
            0x65, 0x72, 0x20, 0x68, 0x61, 0x6e, 0x64, 0x6c, 0x65, 0x73, 0x20, 0x61, 0x70, 0x70,
            0x72, 0x6f, 0x76, 0x61, 0x6c, 0x20, 0x72, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x73,
            0x20, 0x72, 0x65, 0x71, 0x75, 0x69, 0x72, 0x65, 0x64, 0x20, 0x62, 0x79, 0x20, 0x63,
            0x6f, 0x6d, 0x6d, 0x61, 0x6e, 0x64, 0x20, 0x72, 0x75, 0x6c, 0x65, 0x73, 0x20, 0x61,
            0x6e, 0x64, 0x20, 0x70, 0x6f, 0x6c, 0x69, 0x63, 0x79, 0x2e, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        ],
    ),
    // Permission description.
    (
        0xc0a76d8,
        &[
            0x4f, 0x6e, 0x6c, 0x79, 0x20, 0x61, 0x73, 0x6b, 0x20, 0x66, 0x6f, 0x72, 0x20, 0x61,
            0x63, 0x74, 0x69, 0x6f, 0x6e, 0x73, 0x20, 0x64, 0x65, 0x74, 0x65, 0x63, 0x74, 0x65,
            0x64, 0x20, 0x61, 0x73, 0x20, 0x70, 0x6f, 0x74, 0x65, 0x6e, 0x74, 0x69, 0x61, 0x6c,
            0x6c, 0x79, 0x20, 0x75, 0x6e, 0x73, 0x61, 0x66, 0x65,
        ],
        &[
            0x41, 0x75, 0x74, 0x6f, 0x2d, 0x72, 0x65, 0x76, 0x69, 0x65, 0x77, 0x20, 0x61, 0x70,
            0x70, 0x72, 0x6f, 0x76, 0x61, 0x6c, 0x20, 0x72, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74,
            0x73, 0x20, 0x77, 0x69, 0x74, 0x68, 0x6f, 0x75, 0x74, 0x20, 0x61, 0x20, 0x73, 0x61,
            0x6e, 0x64, 0x62, 0x6f, 0x78, 0x2e, 0x20, 0x20, 0x20,
        ],
    ),
    // Permission description.
    (
        0xc0b356b,
        &[
            0x43, 0x6f, 0x64, 0x65, 0x78, 0x20, 0x63, 0x61, 0x6e, 0x20, 0x72, 0x65, 0x61, 0x64,
            0x20, 0x61, 0x6e, 0x64, 0x20, 0x65, 0x64, 0x69, 0x74, 0x20, 0x66, 0x69, 0x6c, 0x65,
            0x73, 0x20, 0x69, 0x6e, 0x20, 0x74, 0x68, 0x65, 0x20, 0x63, 0x75, 0x72, 0x72, 0x65,
            0x6e, 0x74, 0x20, 0x77, 0x6f, 0x72, 0x6b, 0x73, 0x70, 0x61, 0x63, 0x65, 0x2c, 0x20,
            0x61, 0x6e, 0x64, 0x20, 0x72, 0x75, 0x6e, 0x20, 0x63, 0x6f, 0x6d, 0x6d, 0x61, 0x6e,
            0x64, 0x73, 0x2e, 0x20, 0x41, 0x70, 0x70, 0x72, 0x6f, 0x76, 0x61, 0x6c, 0x20, 0x69,
            0x73, 0x20, 0x72, 0x65, 0x71, 0x75, 0x69, 0x72, 0x65, 0x64, 0x20, 0x74, 0x6f, 0x20,
            0x61, 0x63, 0x63, 0x65, 0x73, 0x73, 0x20, 0x74, 0x68, 0x65, 0x20, 0x69, 0x6e, 0x74,
            0x65, 0x72, 0x6e, 0x65, 0x74, 0x20, 0x6f, 0x72, 0x20, 0x65, 0x64, 0x69, 0x74, 0x20,
            0x6f, 0x74, 0x68, 0x65, 0x72, 0x20, 0x66, 0x69, 0x6c, 0x65, 0x73, 0x2e, 0x20, 0x28,
            0x49, 0x64, 0x65, 0x6e, 0x74, 0x69, 0x63, 0x61, 0x6c, 0x20, 0x74, 0x6f, 0x20, 0x41,
            0x67, 0x65, 0x6e, 0x74, 0x20, 0x6d, 0x6f, 0x64, 0x65, 0x29,
        ],
        &[
            0x43, 0x6f, 0x64, 0x65, 0x78, 0x20, 0x72, 0x75, 0x6e, 0x73, 0x20, 0x77, 0x69, 0x74,
            0x68, 0x6f, 0x75, 0x74, 0x20, 0x61, 0x20, 0x73, 0x61, 0x6e, 0x64, 0x62, 0x6f, 0x78,
            0x2e, 0x20, 0x41, 0x70, 0x70, 0x72, 0x6f, 0x76, 0x61, 0x6c, 0x20, 0x72, 0x65, 0x71,
            0x75, 0x65, 0x73, 0x74, 0x73, 0x20, 0x66, 0x6f, 0x6c, 0x6c, 0x6f, 0x77, 0x20, 0x63,
            0x6f, 0x6d, 0x6d, 0x61, 0x6e, 0x64, 0x20, 0x72, 0x75, 0x6c, 0x65, 0x73, 0x20, 0x61,
            0x6e, 0x64, 0x20, 0x70, 0x6f, 0x6c, 0x69, 0x63, 0x79, 0x3b, 0x20, 0x74, 0x68, 0x65,
            0x20, 0x73, 0x65, 0x6c, 0x65, 0x63, 0x74, 0x65, 0x64, 0x20, 0x72, 0x65, 0x76, 0x69,
            0x65, 0x77, 0x65, 0x72, 0x20, 0x68, 0x61, 0x6e, 0x64, 0x6c, 0x65, 0x73, 0x20, 0x74,
            0x68, 0x6f, 0x73, 0x65, 0x20, 0x72, 0x65, 0x71, 0x75, 0x65, 0x73, 0x74, 0x73, 0x2e,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
            0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20, 0x20,
        ],
    ),
];

// Official0.163.0 Cargo release; text/rodata file offsets are VA -0x400000.
// The two new UDS strings occupy verified zero alignment padding after .fini.
const PATCHES_163: &[Patch] = &[
    // Local TUI resume retains current permissions; remote and reviewer gates stay intact.
    (0x9fb95a8, b"\xb5\x01\x80\x37", b"\x0d\x00\x00\x14"),
    // UDS canonical FD35 root.
    (0xa47cb54, b"\xe8\x85\x8e\x52\xe0\x63\x00\x91\xa8\x0d\xae\x72\xa1\x00\x80\x52\xff\x73\x00\x39\xe8\x1b\x00\xb9", b"\x20\xd5\x00\xb0\x00\x00\x2f\x91\x1f\x20\x03\xd5\x21\x02\x80\x52\x1f\x20\x03\xd5\x1f\x20\x03\xd5"),
    // UDS UID-only format.
    (0xa47cc30, b"\x80\xd8\x00\xd0\x00\xf0\x11\x91", b"\x20\xd5\x00\xb0\x00\x80\x2f\x91"),
    // UDS path in unmapped-source alignment padding.
    (0xbf21bc0, b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", b"\x2f\x70\x72\x6f\x63\x2f\x73\x65\x6c\x66\x2f\x66\x64\x2f\x33\x35\x00"),
    // UDS UID format in alignment padding.
    (0xbf21be0, b"\x00\x00", b"\xc0\x00"),
    // Auto ID allocation.
    (0xa483db8, b"\x40\x01\x80\x52", b"\x60\x02\x80\x52"),
    // Auto full-access ID copy and lengths.
    (0xa483dd4, b"\x29\xd9\x00\x91\x68\xac\x8c\x52\x29\x01\x40\xf9\xe0\x03\xc0\x3d\x08\x10\x00\x79\x48\x01\x80\x52\x09\x00\x00\xf9", b"\x29\xad\x03\x91\x28\xf1\x40\xb8\x20\x01\xc0\x3d\x08\xf0\x00\xb8\x00\x00\x80\x3d\x68\x02\x80\x52\xe0\x03\xc0\x3d"),
    // Auto Disabled enum and ID length.
    (0xa483df8, b"\xa8\xa3\x00\xd1\x00\x01\x80\x52\xe1\x03\x1f\xaa\xe2\x03\x1f\x2a\xe3\x03\x1f\x2a\xe4\x03\x1f\x2a\x87\x7e\x8f\x97", b"\xa8\x0a\x00\xf9\xe9\x07\x41\xb2\xe9\xb7\x00\xf9\x1f\x20\x03\xd5\x1f\x20\x03\xd5\x1f\x20\x03\xd5\x1f\x20\x03\xd5"),
    // Auto ID OOM allocation layout.
    (0xa483f6c, b"\x41\x01\x80\x52", b"\x61\x02\x80\x52"),
    // Named full-access ID pointer.
    (0x9e72274, b"\x84\x58\x31\x91", b"\x84\x4c\x32\x91"),
    // Named full-access ID pointer.
    (0x9e7236c, b"\x84\x58\x31\x91", b"\x84\x4c\x32\x91"),
    // Named full-access ID length.
    (0x9e7228c, b"\x45\x01\x80\x52", b"\x65\x02\x80\x52"),
    // Named full-access ID length.
    (0x9e72384, b"\x45\x01\x80\x52", b"\x65\x02\x80\x52"),
    // Named inline Auto description tail.
    (0x9e72340, b"\x69\x2e\x8c\x52", b"\x49\xf1\x42\xb8"),
    // Named inline Auto tail immediate removed.
    (0x9e72348, b"\xc9\xac\xac\x72", b"\x1f\x20\x03\xd5"),
    // Legacy inline Auto description tail.
    (0x9e7bcac, b"\x68\x2e\x8c\x52\xc8\xac\xac\x72", b"\x28\xf1\x42\xb8\x1f\x20\x03\xd5"),
    // Auto current marker matches Disabled.
    (0x9e7c9c4, b"\x80\x04\x00\x54", b"\x20\xfc\xff\x54"),
    // Shortcut omits Read Only.
    (0x9e09fe0, b"\xa1\xf8\xff\x54", b"\xc5\xff\xff\x17"),
    // Named popup skips Read Only, preserves scratch pointer.
    (0x9e7260c, b"\x68\xa4\x45\xa9\x3f\x11\x00\xf1", b"\xf9\xa3\x02\x91\x85\x00\x00\x14"),
    // Named popup retains actual vector length.
    (0x9e72838, b"\xa9\x06\x00\x91", b"\xe9\x43\x40\xf9"),
    // Permission description.
    (0xc452b94, b"\x52\x65\x61\x64\x20\x61\x6e\x64\x20\x65\x64\x69\x74\x20\x77\x6f\x72\x6b\x73\x70\x61\x63\x65\x20\x66\x69\x6c\x65\x73\x20\x61\x6e\x64\x20\x72\x75\x6e\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x73\x2c\x20\x77\x69\x74\x68\x20\x61\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x69\x72\x65\x64\x20\x66\x6f\x72\x20\x69\x6e\x74\x65\x72\x6e\x65\x74\x20\x61\x63\x63\x65\x73\x73\x20\x6f\x72\x20\x65\x64\x69\x74\x73\x20\x6f\x75\x74\x73\x69\x64\x65\x20\x74\x68\x65\x20\x77\x6f\x72\x6b\x73\x70\x61\x63\x65", b"\x52\x75\x6e\x20\x77\x69\x74\x68\x6f\x75\x74\x20\x61\x20\x73\x61\x6e\x64\x62\x6f\x78\x3b\x20\x74\x68\x65\x20\x73\x65\x6c\x65\x63\x74\x65\x64\x20\x72\x65\x76\x69\x65\x77\x65\x72\x20\x68\x61\x6e\x64\x6c\x65\x73\x20\x61\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x65\x73\x74\x73\x20\x72\x65\x71\x75\x69\x72\x65\x64\x20\x62\x79\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x20\x72\x75\x6c\x65\x73\x20\x61\x6e\x64\x20\x70\x6f\x6c\x69\x63\x79\x2e\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20"),
    // Permission description.
    (0xc487047, b"\x43\x6f\x64\x65\x78\x20\x63\x61\x6e\x20\x72\x65\x61\x64\x20\x61\x6e\x64\x20\x65\x64\x69\x74\x20\x66\x69\x6c\x65\x73\x20\x69\x6e\x20\x74\x68\x65\x20\x63\x75\x72\x72\x65\x6e\x74\x20\x77\x6f\x72\x6b\x73\x70\x61\x63\x65\x2c\x20\x61\x6e\x64\x20\x72\x75\x6e\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x73\x2e\x20\x41\x70\x70\x72\x6f\x76\x61\x6c\x20\x69\x73\x20\x72\x65\x71\x75\x69\x72\x65\x64\x20\x74\x6f\x20\x61\x63\x63\x65\x73\x73\x20\x74\x68\x65\x20\x69\x6e\x74\x65\x72\x6e\x65\x74\x20\x6f\x72\x20\x65\x64\x69\x74\x20\x6f\x74\x68\x65\x72\x20\x66\x69\x6c\x65\x73\x2e\x20\x28\x49\x64\x65\x6e\x74\x69\x63\x61\x6c\x20\x74\x6f\x20\x41\x67\x65\x6e\x74\x20\x6d\x6f\x64\x65\x29", b"\x43\x6f\x64\x65\x78\x20\x72\x75\x6e\x73\x20\x77\x69\x74\x68\x6f\x75\x74\x20\x61\x20\x73\x61\x6e\x64\x62\x6f\x78\x2e\x20\x41\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x65\x73\x74\x73\x20\x66\x6f\x6c\x6c\x6f\x77\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x20\x72\x75\x6c\x65\x73\x20\x61\x6e\x64\x20\x70\x6f\x6c\x69\x63\x79\x3b\x20\x74\x68\x65\x20\x73\x65\x6c\x65\x63\x74\x65\x64\x20\x72\x65\x76\x69\x65\x77\x65\x72\x20\x68\x61\x6e\x64\x6c\x65\x73\x20\x74\x68\x6f\x73\x65\x20\x72\x65\x71\x75\x65\x73\x74\x73\x2e\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20"),
    // Permission description.
    (0xc452c60, b"\x4f\x6e\x6c\x79\x20\x61\x73\x6b\x20\x66\x6f\x72\x20\x61\x63\x74\x69\x6f\x6e\x73\x20\x64\x65\x74\x65\x63\x74\x65\x64\x20\x61\x73\x20\x70\x6f\x74\x65\x6e\x74\x69\x61\x6c\x6c\x79\x20\x75\x6e\x73\x61\x66\x65", b"\x41\x75\x74\x6f\x2d\x72\x65\x76\x69\x65\x77\x20\x61\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x65\x73\x74\x73\x20\x77\x69\x74\x68\x6f\x75\x74\x20\x61\x20\x73\x61\x6e\x64\x62\x6f\x78\x2e\x20\x20\x20"),
];

// Official0.164.0 Cargo release; exact raw digest, guarded AArch64 instructions
// and zero .fini alignment padding. Approval review uses native Untrusted.
const PATCHES_164: &[Patch] = &[
    // Retain local resume permissions; remote unchanged
    (0xa0d6a98, b"\x08\x01\x39\x0a", b"\x28\x03\x00\x52"),
    // Retain local resume permissions; remote unchanged
    (0xa0d6abc, b"\x08\x01\x39\x0a", b"\x28\x03\x00\x52"),
    // UDS FD35 root
    (0xac4a7d4, b"\xe8\x85\x8e\x52\xe0\x63\x00\x91\xa8\x0d\xae\x72\xa1\x00\x80\x52\xff\x73\x00\x39\xe8\x1b\x00\xb9", b"\x20\xd8\x00\xb0\x00\xc0\x08\x91\x1f\x20\x03\xd5\x21\x02\x80\x52\x1f\x20\x03\xd5\x1f\x20\x03\xd5"),
    // UDS UID format
    (0xac4a8b8, b"\xa0\xdb\x00\xb0\x00\x18\x21\x91", b"\x20\xd8\x00\xb0\x00\x40\x09\x91"),
    // UDS root in verified alignment padding
    (0xc74f230, b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00", b"\x2f\x70\x72\x6f\x63\x2f\x73\x65\x6c\x66\x2f\x66\x64\x2f\x33\x35\x00"),
    // UDS UID format padding
    (0xc74f250, b"\x00\x00", b"\xc0\x00"),
    // Auto full-access ID allocation
    (0xac51484, b"\x40\x01\x80\x52", b"\x60\x02\x80\x52"),
    // Auto full-access ID copy
    (0xac514a0, b"\x29\x89\x15\x91\x68\xac\x8c\x52\x29\x01\x40\xf9\xe0\x03\xc0\x3d\x08\x10\x00\x79\x48\x01\x80\x52\x09\x00\x00\xf9", b"\x29\x5d\x18\x91\x28\xf1\x40\xb8\x20\x01\xc0\x3d\x08\xf0\x00\xb8\x00\x00\x80\x3d\x68\x02\x80\x52\xe0\x03\xc0\x3d"),
    // Auto Disabled profile and length
    (0xac514c4, b"\xa8\xa3\x00\xd1\x00\x01\x80\x52\xe1\x03\x1f\xaa\xe2\x03\x1f\x2a\xe3\x03\x1f\x2a\xe4\x03\x1f\x2a\x8f\x66\x8c\x97", b"\xa8\x0a\x00\xf9\x09\x00\xf0\xd2\xe9\xb7\x00\xf9\x1f\x20\x03\xd5\x1f\x20\x03\xd5\x1f\x20\x03\xd5\x1f\x20\x03\xd5"),
    // Auto OOM allocation length
    (0xac51638, b"\x41\x01\x80\x52", b"\x61\x02\x80\x52"),
    // Named full-access profile ID
    (0x9f4c534, b"\x84\xdc\x3b\x91", b"\x84\xd0\x3c\x91"),
    // Named full-access profile ID
    (0x9f4c628, b"\x84\xdc\x3b\x91", b"\x84\xd0\x3c\x91"),
    // Named full-access profile ID length
    (0x9f4c54c, b"\x45\x01\x80\x52", b"\x65\x02\x80\x52"),
    // Named full-access profile ID length
    (0x9f4c640, b"\x45\x01\x80\x52", b"\x65\x02\x80\x52"),
    // Named Auto description inline tail
    (0x9f4c5fc, b"\x69\x2e\x8c\x52", b"\x49\xf1\x42\xb8"),
    // Named Auto old inline tail immediate
    (0x9f4c604, b"\xc9\xac\xac\x72", b"\x1f\x20\x03\xd5"),
    // Legacy Auto description inline tail
    (0x9f534b4, b"\x68\x2e\x8c\x52\xc8\xac\xac\x72", b"\x28\xf1\x42\xb8\x1f\x20\x03\xd5"),
    // Auto current marker matches Disabled
    (0x9f541d4, b"\xc0\x04\x00\x54", b"\x20\xfc\xff\x54"),
    // Named popup omits Read Only
    (0x9f4c834, b"\xa8\x32\x40\xf9\x1f\x11\x00\xf1", b"\xf5\x2b\x40\xf9\x6d\x00\x00\x14"),
    // Named popup retains vector length
    (0x9f4ca00, b"\xa9\x06\x00\x91", b"\xe9\x03\x15\xaa"),
    // Shortcut omits Read Only
    (0x9f0afcc, b"\x81\xfa\xff\x54", b"\xd4\xff\xff\x17"),
    // Experiment Auto ID allocation
    (0xa0dcb98, b"\x40\x01\x80\x52", b"\x60\x02\x80\x52"),
    // Experiment Auto full-access ID; OnRequest unchanged
    (0xa0dcba8, b"\x49\x5d\x01\xd0\x29\xdd\x3b\x91\x68\xac\x8c\x52\x29\x01\x40\xf9\x2a\x60\x80\x52\x08\x10\x00\x79\x48\x01\x80\x52\x6a\x62\x00\x79\x09\x00\x00\xf9", b"\x49\x5d\x01\xd0\x29\xd1\x3c\x91\x28\xf1\x40\xb8\x20\x01\xc0\x3d\x08\xf0\x00\xb8\x00\x00\x80\x3d\x68\x02\x80\x52\x2a\x60\x80\x52\x6a\x62\x00\x79"),
    // Experiment Auto ID lengths and no runtime profile
    (0xa0dcbcc, b"\xe9\x39\x01\xb0\x20\xc1\xc0\x3d\x68\x02\x00\xa9\x60\x06\x80\x3d", b"\x09\x00\x80\x92\x68\x02\x00\xa9\x68\x26\x01\xa9\x1f\x20\x03\xd5"),
    // Experiment Auto OOM length
    (0xa0dcbec, b"\x41\x01\x80\x52", b"\x61\x02\x80\x52"),
    // Permission description
    (0xcc8f191, b"\x52\x65\x61\x64\x20\x61\x6e\x64\x20\x65\x64\x69\x74\x20\x77\x6f\x72\x6b\x73\x70\x61\x63\x65\x20\x66\x69\x6c\x65\x73\x20\x61\x6e\x64\x20\x72\x75\x6e\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x73\x2c\x20\x77\x69\x74\x68\x20\x61\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x69\x72\x65\x64\x20\x66\x6f\x72\x20\x69\x6e\x74\x65\x72\x6e\x65\x74\x20\x61\x63\x63\x65\x73\x73\x20\x6f\x72\x20\x65\x64\x69\x74\x73\x20\x6f\x75\x74\x73\x69\x64\x65\x20\x74\x68\x65\x20\x77\x6f\x72\x6b\x73\x70\x61\x63\x65", b"\x52\x75\x6e\x20\x77\x69\x74\x68\x6f\x75\x74\x20\x61\x20\x73\x61\x6e\x64\x62\x6f\x78\x3b\x20\x74\x68\x65\x20\x73\x65\x6c\x65\x63\x74\x65\x64\x20\x72\x65\x76\x69\x65\x77\x65\x72\x20\x68\x61\x6e\x64\x6c\x65\x73\x20\x61\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x65\x73\x74\x73\x20\x72\x65\x71\x75\x69\x72\x65\x64\x20\x62\x79\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x20\x72\x75\x6c\x65\x73\x20\x61\x6e\x64\x20\x70\x6f\x6c\x69\x63\x79\x2e\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20"),
    // Permission description
    (0xcccf573, b"\x43\x6f\x64\x65\x78\x20\x63\x61\x6e\x20\x72\x65\x61\x64\x20\x61\x6e\x64\x20\x65\x64\x69\x74\x20\x66\x69\x6c\x65\x73\x20\x69\x6e\x20\x74\x68\x65\x20\x63\x75\x72\x72\x65\x6e\x74\x20\x77\x6f\x72\x6b\x73\x70\x61\x63\x65\x2c\x20\x61\x6e\x64\x20\x72\x75\x6e\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x73\x2e\x20\x41\x70\x70\x72\x6f\x76\x61\x6c\x20\x69\x73\x20\x72\x65\x71\x75\x69\x72\x65\x64\x20\x74\x6f\x20\x61\x63\x63\x65\x73\x73\x20\x74\x68\x65\x20\x69\x6e\x74\x65\x72\x6e\x65\x74\x20\x6f\x72\x20\x65\x64\x69\x74\x20\x6f\x74\x68\x65\x72\x20\x66\x69\x6c\x65\x73\x2e\x20\x28\x49\x64\x65\x6e\x74\x69\x63\x61\x6c\x20\x74\x6f\x20\x41\x67\x65\x6e\x74\x20\x6d\x6f\x64\x65\x29", b"\x43\x6f\x64\x65\x78\x20\x72\x75\x6e\x73\x20\x77\x69\x74\x68\x6f\x75\x74\x20\x61\x20\x73\x61\x6e\x64\x62\x6f\x78\x2e\x20\x41\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x65\x73\x74\x73\x20\x66\x6f\x6c\x6c\x6f\x77\x20\x63\x6f\x6d\x6d\x61\x6e\x64\x20\x72\x75\x6c\x65\x73\x20\x61\x6e\x64\x20\x70\x6f\x6c\x69\x63\x79\x3b\x20\x74\x68\x65\x20\x73\x65\x6c\x65\x63\x74\x65\x64\x20\x72\x65\x76\x69\x65\x77\x65\x72\x20\x68\x61\x6e\x64\x6c\x65\x73\x20\x74\x68\x6f\x73\x65\x20\x72\x65\x71\x75\x65\x73\x74\x73\x2e\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20\x20"),
    // Permission description
    (0xcc86f01, b"\x4f\x6e\x6c\x79\x20\x61\x73\x6b\x20\x66\x6f\x72\x20\x61\x63\x74\x69\x6f\x6e\x73\x20\x64\x65\x74\x65\x63\x74\x65\x64\x20\x61\x73\x20\x70\x6f\x74\x65\x6e\x74\x69\x61\x6c\x6c\x79\x20\x75\x6e\x73\x61\x66\x65", b"\x41\x75\x74\x6f\x2d\x72\x65\x76\x69\x65\x77\x20\x61\x70\x70\x72\x6f\x76\x61\x6c\x20\x72\x65\x71\x75\x65\x73\x74\x73\x20\x77\x69\x74\x68\x6f\x75\x74\x20\x61\x20\x73\x61\x6e\x64\x62\x6f\x78\x2e\x20\x20\x20"),
];

pub(super) const ADAPTIVE_POLICY: &str = "termux-fd-remap-99b-v1";
pub(super) const ADAPTIVE_UDS_POLICY: &str = "termux-uds-fd35-adaptive-v1";
const FD35_BYTES: &[u8] = b"/proc/self/fd/35\0";
const FMT_UID_ONLY: &[u8] = b"\xc0\0";
const DAEMON_FMT_NEEDLE: &[u8] = b"\x0dcodex-daemon-\xc0\0";
const NOP_INSN: u32 = 0xd503_201f;

// Publication metadata is shared by adaptation, descriptor writing and admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Policy {
    pub id: &'static str,
    pub version: &'static str,
    pub raw_sha256: Option<&'static str>,
    pub archive_sha256: Option<&'static str>,
    pub uds: &'static str,
    pub permission: Option<&'static str>,
    pub changed_bytes: Option<usize>,
}

pub(super) fn policy(id: &str) -> Option<Policy> {
    match id {
        POLICY | PREVIOUS_POLICY => Some(Policy {
            id: if id == POLICY {
                POLICY
            } else {
                PREVIOUS_POLICY
            },
            version: "0.160.0",
            raw_sha256: Some(RAW_SHA256),
            archive_sha256: None,
            uds: "termux-uds-0-160-0-v1",
            permission: (id == POLICY).then_some(PERMISSION_POLICY),
            changed_bytes: Some(if id == POLICY { CHANGED_BYTES } else { 43 }),
        }),
        "termux-fd-remap-v4" => Some(Policy {
            id: "termux-fd-remap-v4",
            version: "0.161.0",
            raw_sha256: Some("0129f94f4f1197bd75f6c4265889061f386ed302bbc50c6e57e3e0660dfb28a7"),
            archive_sha256: Some(
                "3c02e2ae34be0d06e62557e98fc5c0a783bec5a2fed406fe00e565803bf84ee8",
            ),
            uds: "termux-uds-0-161-0-v1",
            permission: Some("termux-permission-picker-0-161-0-v1"),
            changed_bytes: Some(432),
        }),
        "termux-fd-remap-v5" => Some(Policy {
            id: "termux-fd-remap-v5",
            version: "0.163.0",
            raw_sha256: Some("8a96f66d059d653e68bfd5f3c79337e6f1d7d170d80bf705eb7d0249b34c0ece"),
            archive_sha256: Some(
                "9d8ecf2d19d3e184877983902d6c85de48b3888dcb72c382a8a0c975845927fb",
            ),
            uds: "termux-uds-0-163-0-v1",
            permission: Some("termux-permission-picker-0-163-0-v1"),
            changed_bytes: Some(433),
        }),
        "termux-fd-remap-v6" => Some(Policy {
            id: "termux-fd-remap-v6",
            version: "0.164.0",
            raw_sha256: Some("8813c8d8a4cea2eeba271b72ab22f49628b184b24ab18dbef06b4ce41e9a8d39"),
            archive_sha256: Some(
                "5918add6c7e9cf1e14a9af1b4c0f4bf1d24a43903faac26df5dc4c019ea91bd5",
            ),
            uds: "termux-uds-0-164-0-v1",
            permission: Some("termux-permission-picker-0-164-0-v1"),
            changed_bytes: Some(478),
        }),
        ADAPTIVE_POLICY => Some(Policy {
            id: ADAPTIVE_POLICY,
            version: "0.165.0+",
            raw_sha256: None,
            archive_sha256: None,
            uds: ADAPTIVE_UDS_POLICY,
            permission: None,
            changed_bytes: None,
        }),
        _ => None,
    }
}

pub(super) fn required(version: &str) -> Result<Option<Policy>, &'static str> {
    let parts = version
        .split('.')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "unqualified upstream UDS version")?;
    if parts.len() != 3 {
        return Err("upstream UDS version requires fresh Termux qualification");
    }
    match parts.as_slice() {
        [0, 160, 0] => Ok(policy(POLICY)),
        [0, 161, 0] => Ok(policy("termux-fd-remap-v4")),
        [0, 163, 0] => Ok(policy("termux-fd-remap-v5")),
        [0, 164, 0] => Ok(policy("termux-fd-remap-v6")),
        [0, minor, _] if *minor >= 165 => Ok(policy(ADAPTIVE_POLICY)),
        [major, _, _] if *major >= 1 => Ok(policy(ADAPTIVE_POLICY)),
        older if older < [0, 160, 0].as_slice() => Ok(None),
        _ => Err("upstream UDS version requires fresh Termux qualification"),
    }
}

pub(super) fn supports_version(id: &str, version: &str) -> bool {
    if id == super::PATCH_POLICY_ID {
        required(version).is_ok_and(|policy| policy.is_none_or(|policy| policy.id == POLICY))
    } else if id == ADAPTIVE_POLICY {
        required(version).is_ok_and(|opt| opt.is_some_and(|p| p.id == ADAPTIVE_POLICY))
    } else {
        policy(id).is_some_and(|policy| policy.version == version)
    }
}

fn blocks(policy: Policy) -> (usize, &'static [Patch]) {
    if policy.id == "termux-fd-remap-v6" {
        (0xc7bf846, PATCHES_164)
    } else if policy.id == "termux-fd-remap-v5" {
        (0xbf8e47c, PATCHES_163)
    } else if policy.id == "termux-fd-remap-v4" {
        (0xbbc9acf, PATCHES_161)
    } else {
        (0xb8cc2c4, PATCHES)
    }
}

pub(super) fn apply(
    bytes: &mut [u8],
    raw_digest: &str,
    policy: Policy,
) -> Result<usize, &'static str> {
    if policy.id == ADAPTIVE_POLICY {
        return apply_adaptive_uds_fd35(bytes);
    }
    let (guard, patches) = blocks(policy);
    if policy.permission.is_none()
        || policy.raw_sha256 != Some(raw_digest)
        || bytes.get(guard..guard + 16) != Some(DAEMON_FMT_NEEDLE)
        || patches.iter().any(|(off, old, new)| {
            old.len() != new.len() || bytes.get(*off..off + old.len()) != Some(*old)
        })
    {
        return Err("upstream artifact does not match qualified Termux policy");
    }
    let changed: usize = patches
        .iter()
        .map(|(_, old, new)| old.iter().zip(*new).filter(|(a, b)| a != b).count())
        .sum();
    if policy.changed_bytes != Some(changed) {
        return Err("qualified Termux changed-byte count is inconsistent");
    }
    for (off, _, new) in patches {
        bytes[*off..off + new.len()].copy_from_slice(new);
    }
    Ok(changed)
}

#[derive(Clone, Copy, Debug)]
struct ElfSection {
    off: usize,
    sz: usize,
}

#[derive(Clone, Copy, Debug)]
struct ElfLoad {
    flags: u32,
    off: usize,
    filesz: usize,
    delta: i128,
}

#[derive(Clone, Copy, Debug)]
struct ParsedElf {
    text: ElfSection,
    fini: ElfSection,
    rodata: ElfSection,
    rx_load: ElfLoad,
    ro_delta: i128,
}

#[derive(Clone, Copy, Debug)]
struct RxCave {
    fd35_off: usize,
    fmt_off: usize,
}

#[derive(Clone, Debug)]
struct Site1Match {
    start_off: usize,
    span_insns: usize,
    ptr_reg: u8,
    len_reg: u8,
    interleaved: Vec<u32>,
}

#[derive(Clone, Copy, Debug)]
struct Site2Match {
    adrp_off: usize,
    add_off: usize,
    fmt_reg: u8,
}

fn le_u16(bytes: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(
        bytes.get(off..off + 2)?.try_into().ok()?,
    ))
}

fn le_u32(bytes: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(
        bytes.get(off..off + 4)?.try_into().ok()?,
    ))
}

fn le_u64(bytes: &[u8], off: usize) -> Option<u64> {
    Some(u64::from_le_bytes(
        bytes.get(off..off + 8)?.try_into().ok()?,
    ))
}

fn off_to_va(off: usize, delta: i128) -> Result<u64, &'static str> {
    u64::try_from(off as i128 + delta).map_err(|_| "ELF virtual address overflow")
}

fn encode_adrp(rd: u8, pc_va: u64, target_va: u64) -> Result<u32, &'static str> {
    let target_page = i128::from(target_va & !0xfff);
    let pc_page = i128::from(pc_va & !0xfff);
    let page_diff = (target_page - pc_page) >> 12;
    if !(-(1 << 20)..(1 << 20)).contains(&page_diff) {
        return Err("ADRP page offset out of +/-4GiB range");
    }
    let imm = (page_diff as u32) & 0x1f_ffff;
    let immlo = imm & 0x3;
    let immhi = (imm >> 2) & 0x7_ffff;
    Ok(0x9000_0000 | (immlo << 29) | (immhi << 5) | u32::from(rd & 0x1f))
}

fn encode_add_imm12(rd: u8, rn: u8, imm12: u16) -> Result<u32, &'static str> {
    if imm12 >= 0x1000 {
        return Err("ADD imm12 out of 12-bit range");
    }
    Ok(0x9100_0000 | (u32::from(imm12) << 10) | (u32::from(rn & 0x1f) << 5) | u32::from(rd & 0x1f))
}

fn encode_movz_w(rd: u8, imm16: u16) -> u32 {
    0x5280_0000 | (u32::from(imm16) << 5) | u32::from(rd & 0x1f)
}

fn is_pc_relative(insn: u32) -> bool {
    let op0 = (insn >> 25) & 0xf;
    matches!(insn & 0x9f00_0000, 0x1000_0000 | 0x9000_0000) // ADR / ADRP
        || (insn & 0x3b00_0000) == 0x1800_0000 // LDR / PRFM literal
        || (insn & 0x7c00_0000) == 0x1400_0000 // B / BL
        || matches!(op0, 0b1010 | 0b1011) // B.cond / CBZ / CBNZ / TBZ / TBNZ
}

fn is_branch_or_call(insn: u32) -> bool {
    let op0 = (insn >> 25) & 0xf;
    (insn & 0x7c00_0000) == 0x1400_0000 // B / BL
        || (insn & 0xfe00_0000) == 0xd600_0000 // BR / BLR / RET
        || matches!(op0, 0b1010 | 0b1011)
}

fn is_mov_wide(insn: u32) -> bool {
    matches!((insn >> 23) & 0x3f, 0b100101 | 0b000101 | 0b110101)
}

fn is_safe_interleaved(insn: u32) -> bool {
    if insn == NOP_INSN {
        return true;
    }
    if is_pc_relative(insn)
        || is_branch_or_call(insn)
        || (insn & 0xff00_0000) == 0xd400_0000 // exception/sys
        || (insn & 0xff00_0000) == 0xd500_0000
    // MSR/MRS/HINT/BARRIER
    {
        return false;
    }
    let op0 = (insn >> 25) & 0xf;
    // Reject flag-setting DP instructions (ADDS/SUBS/ANDS/...) to keep NZCV untouched.
    if (op0 & 0b1110) == 0b1000 && !is_mov_wide(insn) && (insn & (1 << 29)) != 0 {
        return false;
    }
    if (op0 & 0b0111) == 0b0101 && (insn & (1 << 29)) != 0 && (insn & 0x1f00_0000) != 0x1a00_0000 {
        return false;
    }
    // Reject SP writes via unflagged ADD/SUB (imm) where Rd == 31.
    if (insn & 0x1f00_0000) == 0x1100_0000 && (insn & 0x1f) == 31 {
        return false;
    }
    // Reject reading or writing/storing X0/W0 or X1/W1 cleanly.
    let rt_or_rd = insn & 0x1f;
    if rt_or_rd <= 1 {
        return false;
    }
    if !is_mov_wide(insn) && ((insn >> 5) & 0x1f) <= 1 {
        return false;
    }
    // Data processing (register): also reject Rm in {0, 1}.
    if (op0 & 0b0111) == 0b0101 && ((insn >> 16) & 0x1f) <= 1 {
        return false;
    }
    // Load/store pair: reject Rt2 in {0, 1} and writeback on SP.
    if (insn & 0x3a00_0000) == 0x2800_0000 {
        if ((insn >> 10) & 0x1f) <= 1 {
            return false;
        }
        let writeback_mode = (insn >> 23) & 0x3;
        if matches!(writeback_mode, 1 | 3) && ((insn >> 5) & 0x1f) == 31 {
            return false;
        }
    }
    true
}

fn parse_elf(data: &[u8]) -> Result<ParsedElf, &'static str> {
    if data.len() < 64
        || &data[..4] != b"\x7fELF"
        || data[4] != 2
        || data[5] != 1
        || data[6] != 1
        || le_u16(data, 16) != Some(2)
        || le_u16(data, 18) != Some(183)
        || le_u32(data, 20) != Some(1)
        || le_u16(data, 52) != Some(64)
        || le_u16(data, 54) != Some(56)
    {
        return Err("runtime is not an ELF64 LE AArch64 ET_EXEC binary");
    }
    let e_phoff =
        usize::try_from(le_u64(data, 32).ok_or("corrupt ELF")?).map_err(|_| "ELF overflow")?;
    let e_shoff =
        usize::try_from(le_u64(data, 40).ok_or("corrupt ELF")?).map_err(|_| "ELF overflow")?;
    let e_phnum = usize::from(le_u16(data, 56).ok_or("corrupt ELF")?);
    let e_shentsize = usize::from(le_u16(data, 58).ok_or("corrupt ELF")?);
    let e_shnum = usize::from(le_u16(data, 60).ok_or("corrupt ELF")?);
    let e_shstrndx = usize::from(le_u16(data, 62).ok_or("corrupt ELF")?);
    if e_phnum == 0 || e_shentsize < 64 || e_shnum == 0 || e_shstrndx >= e_shnum {
        return Err("corrupt ELF program or section header table");
    }
    let ph_end = e_phnum
        .checked_mul(56)
        .and_then(|sz| e_phoff.checked_add(sz))
        .filter(|&end| end <= data.len())
        .ok_or("ELF program headers out of bounds")?;
    let _ = ph_end;
    let sh_end = e_shnum
        .checked_mul(e_shentsize)
        .and_then(|sz| e_shoff.checked_add(sz))
        .filter(|&end| end <= data.len())
        .ok_or("ELF section headers out of bounds")?;
    let _ = sh_end;

    let mut loads = Vec::new();
    for i in 0..e_phnum {
        let ph = e_phoff + i * 56;
        let p_type = le_u32(data, ph).ok_or("corrupt ELF phdr")?;
        if p_type == 3 {
            return Err("executable has PT_INTERP");
        }
        if p_type == 1 {
            let flags = le_u32(data, ph + 4).ok_or("corrupt ELF phdr")?;
            let off = usize::try_from(le_u64(data, ph + 8).ok_or("corrupt ELF phdr")?)
                .map_err(|_| "ELF phdr offset overflow")?;
            let va = le_u64(data, ph + 16).ok_or("corrupt ELF phdr")?;
            let filesz = usize::try_from(le_u64(data, ph + 32).ok_or("corrupt ELF phdr")?)
                .map_err(|_| "ELF phdr size overflow")?;
            if off.checked_add(filesz).is_none_or(|end| end > data.len()) {
                return Err("ELF PT_LOAD exceeds file bounds");
            }
            loads.push(ElfLoad {
                flags,
                off,
                filesz,
                delta: i128::from(va) - off as i128,
            });
        }
    }

    let shstr_hdr = e_shoff + e_shstrndx * e_shentsize;
    let shstr_off = usize::try_from(le_u64(data, shstr_hdr + 24).ok_or("corrupt shstrtab")?)
        .map_err(|_| "shstrtab offset overflow")?;
    let shstr_sz = usize::try_from(le_u64(data, shstr_hdr + 32).ok_or("corrupt shstrtab")?)
        .map_err(|_| "shstrtab size overflow")?;
    let shstr_end = shstr_off
        .checked_add(shstr_sz)
        .filter(|&end| end <= data.len())
        .ok_or("ELF .shstrtab out of bounds")?;
    let shstr = &data[shstr_off..shstr_end];

    let mut text = None;
    let mut fini = None;
    let mut rodata = None;
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_name = usize::try_from(le_u32(data, sh).ok_or("corrupt shdr")?)
            .map_err(|_| "section name overflow")?;
        let sh_type = le_u32(data, sh + 4).ok_or("corrupt shdr")?;
        let off = usize::try_from(le_u64(data, sh + 24).ok_or("corrupt shdr")?)
            .map_err(|_| "section offset overflow")?;
        let sz = usize::try_from(le_u64(data, sh + 32).ok_or("corrupt shdr")?)
            .map_err(|_| "section size overflow")?;
        if sh_name >= shstr.len() {
            return Err("section name index out of .shstrtab bounds");
        }
        let name_end = shstr[sh_name..]
            .iter()
            .position(|&b| b == 0)
            .map(|rel| sh_name + rel)
            .unwrap_or(shstr.len());
        let name = &shstr[sh_name..name_end];
        if matches!(name, b".text" | b".fini" | b".rodata") {
            if sh_type == 8 || off.checked_add(sz).is_none_or(|end| end > data.len()) {
                return Err("required ELF section is SHT_NOBITS or out of bounds");
            }
            let sec = ElfSection { off, sz };
            match name {
                b".text" => text = Some(sec),
                b".fini" => fini = Some(sec),
                b".rodata" => rodata = Some(sec),
                _ => {}
            }
        }
    }

    let text = text.ok_or("required ELF section .text missing")?;
    let fini = fini.ok_or("required ELF section .fini missing")?;
    let rodata = rodata.ok_or("required ELF section .rodata missing")?;
    if text.off & 3 != 0 || text.sz < 8 || rodata.sz < DAEMON_FMT_NEEDLE.len() {
        return Err("required ELF section dimensions are invalid");
    }
    let rx_matches: Vec<_> = loads
        .iter()
        .copied()
        .filter(|l| {
            (l.flags & 5) == 5
                && l.off <= text.off
                && text.off + text.sz <= l.off + l.filesz
                && l.off <= fini.off
                && fini.off + fini.sz <= l.off + l.filesz
        })
        .collect();
    if rx_matches.len() != 1 {
        return Err("failed to identify unique RX PT_LOAD segment");
    }
    let ro_matches: Vec<_> = loads
        .iter()
        .copied()
        .filter(|l| {
            (l.flags & 4) == 4 && l.off <= rodata.off && rodata.off + rodata.sz <= l.off + l.filesz
        })
        .collect();
    if ro_matches.len() != 1 {
        return Err("failed to identify unique readable PT_LOAD segment for .rodata");
    }

    Ok(ParsedElf {
        text,
        fini,
        rodata,
        rx_load: rx_matches[0],
        ro_delta: ro_matches[0].delta,
    })
}

fn locate_rx_cave(data: &[u8], elf: &ParsedElf) -> Result<RxCave, &'static str> {
    let fini_end = elf
        .fini
        .off
        .checked_add(elf.fini.sz)
        .ok_or("ELF .fini overflow")?;
    let mut rx_end = elf.rx_load.off + elf.rx_load.filesz;
    if elf.rodata.off >= fini_end {
        rx_end = rx_end.min(elf.rodata.off);
    }
    let start_cand = (fini_end.checked_add(16).ok_or("cave overflow")? + 15) & !15;
    let mut cand = start_cand;
    while cand.checked_add(64).is_some_and(|end| end <= rx_end) {
        if data[cand..cand + 64].iter().all(|&b| b == 0) {
            return Ok(RxCave {
                fd35_off: cand,
                fmt_off: cand + 0x20,
            });
        }
        cand += 16;
    }
    Err("insufficient 64-byte all-zero RX alignment cave between .fini and .rodata")
}

fn find_adrp_add_xrefs(
    data: &[u8],
    text_off: usize,
    text_end: usize,
    va_delta: i128,
    target_va: u64,
) -> Vec<(usize, usize, u8)> {
    let lo12 = (target_va & 0xfff) as u32;
    let target_page = i128::from(target_va & !0xfff);
    let mut hits = Vec::new();
    let mut add_off = text_off + 4;
    while add_off + 4 <= text_end {
        let Some(insn1) = le_u32(data, add_off) else {
            break;
        };
        if (insn1 & 0xffc0_0000) == 0x9100_0000 && ((insn1 >> 10) & 0xfff) == lo12 {
            let rn = (insn1 >> 5) & 0x1f;
            let rd = insn1 & 0x1f;
            if rn == rd && rd != 31 {
                for step in 1usize..=5 {
                    let Some(adrp_off) = add_off.checked_sub(step * 4) else {
                        break;
                    };
                    if adrp_off < text_off {
                        break;
                    }
                    let Some(insn0) = le_u32(data, adrp_off) else {
                        break;
                    };
                    if (insn0 & 0x9f00_001f) == (0x9000_0000 | rn) {
                        let immlo = i128::from((insn0 >> 29) & 0x3);
                        let immhi = i128::from((insn0 >> 5) & 0x7_ffff);
                        let mut imm = (immhi << 2) | immlo;
                        if (imm & (1 << 20)) != 0 {
                            imm -= 1 << 21;
                        }
                        let pc_page = (adrp_off as i128 + va_delta) & !0xfff;
                        if pc_page + (imm << 12) == target_page {
                            hits.push((adrp_off, add_off, rd as u8));
                            break;
                        }
                    } else if is_branch_or_call(insn0) {
                        break;
                    }
                }
            }
        }
        add_off += 4;
    }
    hits
}

fn locate_site2(data: &[u8], elf: &ParsedElf) -> Result<Site2Match, &'static str> {
    let ro = &data[elf.rodata.off..elf.rodata.off + elf.rodata.sz];
    let first_rel = ro
        .windows(DAEMON_FMT_NEEDLE.len())
        .position(|w| w == DAEMON_FMT_NEEDLE)
        .ok_or("codex-daemon format string not found in .rodata")?;
    if ro[first_rel + 1..]
        .windows(DAEMON_FMT_NEEDLE.len())
        .any(|w| w == DAEMON_FMT_NEEDLE)
    {
        return Err("expected unique codex-daemon format string in .rodata");
    }
    let fmt_va = off_to_va(elf.rodata.off + first_rel, elf.ro_delta)?;
    let xrefs = find_adrp_add_xrefs(
        data,
        elf.text.off,
        elf.text.off + elf.text.sz,
        elf.rx_load.delta,
        fmt_va,
    );
    if xrefs.len() != 1 {
        return Err("expected unique ADRP+ADD xref to codex-daemon format string");
    }
    let (adrp_off, add_off, fmt_reg) = xrefs[0];
    Ok(Site2Match {
        adrp_off,
        add_off,
        fmt_reg,
    })
}

fn is_str_w_sp(insn: u32, reg: u8, stack_off: u16) -> bool {
    if !stack_off.is_multiple_of(4) {
        return false;
    }
    let imm12 = u32::from(stack_off >> 2);
    if imm12 >= 0x1000 {
        return false;
    }
    insn == (0xb900_03e0 | (imm12 << 10) | u32::from(reg & 0x1f))
}

fn is_strb_wzr_sp(insn: u32, stack_off_plus_4: u16) -> bool {
    if stack_off_plus_4 >= 0x1000 {
        return false;
    }
    insn == (0x3900_03ff | (u32::from(stack_off_plus_4) << 10))
}

fn classify_tmp_span(window: &[u32], start_off: usize) -> Option<Site1Match> {
    let mut movz_idx = None;
    let mut movk_idx = None;
    let mut add_sp_idx = None;
    let mut add_sp_imm = None;
    let mut len5_idx = None;
    let mut mov_w_reg = None;

    for (idx, &insn) in window.iter().enumerate() {
        if (insn & 0xffff_ffe0) == 0x528e_85e0 {
            let reg = (insn & 0x1f) as u8;
            if reg == 31 || movz_idx.is_some() {
                return None;
            }
            movz_idx = Some(idx);
            mov_w_reg = Some(reg);
        } else if (insn & 0xffff_ffe0) == 0x72ae_0da0 {
            let reg = (insn & 0x1f) as u8;
            if reg == 31 || movk_idx.is_some() {
                return None;
            }
            if mov_w_reg.is_some_and(|expected| expected != reg) {
                return None;
            }
            mov_w_reg = Some(reg);
            movk_idx = Some(idx);
        } else if (insn & 0xffc0_03ff) == 0x9100_03e0 {
            if add_sp_idx.is_some() {
                return None;
            }
            add_sp_idx = Some(idx);
            add_sp_imm = Some(((insn >> 10) & 0xfff) as u16);
        } else if insn == 0x5280_00a1 {
            if len5_idx.is_some() {
                return None;
            }
            len5_idx = Some(idx);
        }
        if movz_idx.is_some() && movk_idx.is_some() && add_sp_idx.is_some() && len5_idx.is_some() {
            // Keep scanning for STR/STRB; don't abort early before matching stores.
        }
    }

    let movz_i = movz_idx?;
    let movk_i = movk_idx?;
    let add_i = add_sp_idx?;
    let len5_i = len5_idx?;
    let mov_reg = mov_w_reg?;
    let sp_imm = add_sp_imm?;
    let sp_imm_p4 = sp_imm.checked_add(4)?;

    let mut str_idx = None;
    let mut strb_idx = None;
    for (idx, &insn) in window.iter().enumerate() {
        if is_str_w_sp(insn, mov_reg, sp_imm) {
            if str_idx.is_some() || idx <= movz_i || idx <= movk_i {
                return None;
            }
            str_idx = Some(idx);
        } else if is_strb_wzr_sp(insn, sp_imm_p4) {
            if strb_idx.is_some() {
                return None;
            }
            strb_idx = Some(idx);
        }
    }

    let str_i = str_idx?;
    let strb_i = strb_idx?;
    let dead = [movz_i, movk_i, add_i, len5_i, str_i, strb_i];
    if !dead.contains(&0) {
        return None;
    }
    let last_dead = *dead.iter().max()?;
    let span_insns = last_dead + 1;

    let mut interleaved = Vec::new();
    for (idx, &insn) in window[..span_insns].iter().enumerate() {
        if !dead.contains(&idx) {
            if !is_safe_interleaved(insn) {
                return None;
            }
            interleaved.push(insn);
        }
    }
    if span_insns < 3 + interleaved.len() {
        return None;
    }
    Some(Site1Match {
        start_off,
        span_insns,
        ptr_reg: 0,
        len_reg: 1,
        interleaved,
    })
}

fn locate_site1(
    data: &[u8],
    elf: &ParsedElf,
    site2_off: usize,
) -> Result<Site1Match, &'static str> {
    let win_start = elf.text.off.max(site2_off.saturating_sub(0x200));
    let mut matches: Vec<Site1Match> = Vec::new();
    let mut off = win_start;
    while off + 24 <= site2_off {
        let Some(w0) = le_u32(data, off) else {
            break;
        };
        let is_starter = ((w0 & 0xffff_ffe0) == 0x528e_85e0 && (w0 & 0x1f) != 31)
            || ((w0 & 0xffff_ffe0) == 0x72ae_0da0 && (w0 & 0x1f) != 31)
            || (w0 & 0xffc0_03ff) == 0x9100_03e0
            || w0 == 0x5280_00a1;
        if is_starter {
            let max_cnt = 10.min((site2_off - off) / 4);
            let mut window = Vec::with_capacity(max_cnt);
            for k in 0..max_cnt {
                let wk = le_u32(data, off + k * 4).ok_or("truncated .text window")?;
                if is_branch_or_call(wk) {
                    break;
                }
                window.push(wk);
            }
            if let Some(m) = classify_tmp_span(&window, off) {
                matches.push(m);
            }
        }
        off += 4;
    }
    if matches.len() != 1 {
        return Err("expected unique /tmp UDS root setup site before codex-daemon format");
    }
    Ok(matches.pop().unwrap())
}

pub(super) fn apply_adaptive_uds_fd35(bytes: &mut [u8]) -> Result<usize, &'static str> {
    let elf = parse_elf(bytes)?;
    let cave = locate_rx_cave(bytes, &elf)?;
    let site2 = locate_site2(bytes, &elf)?;
    let site1 = locate_site1(bytes, &elf, site2.adrp_off)?;

    let va_delta = elf.rx_load.delta;
    let fd35_va = off_to_va(cave.fd35_off, va_delta)?;
    let fmt_va = off_to_va(cave.fmt_off, va_delta)?;
    let site1_va = off_to_va(site1.start_off, va_delta)?;
    let site2_va = off_to_va(site2.adrp_off, va_delta)?;

    let mut site1_seq = Vec::with_capacity(site1.span_insns);
    site1_seq.push(encode_adrp(site1.ptr_reg, site1_va, fd35_va)?);
    site1_seq.push(encode_add_imm12(
        site1.ptr_reg,
        site1.ptr_reg,
        (fd35_va & 0xfff) as u16,
    )?);
    site1_seq.push(encode_movz_w(site1.len_reg, FD35_BYTES.len() as u16));
    site1_seq.extend_from_slice(&site1.interleaved);
    site1_seq.resize(site1.span_insns, NOP_INSN);

    let site1_bytes: Vec<u8> = site1_seq.iter().flat_map(|w| w.to_le_bytes()).collect();
    let adrp2 = encode_adrp(site2.fmt_reg, site2_va, fmt_va)?.to_le_bytes();
    let add2 =
        encode_add_imm12(site2.fmt_reg, site2.fmt_reg, (fmt_va & 0xfff) as u16)?.to_le_bytes();

    let mut edits: Vec<(usize, &[u8])> = vec![
        (site1.start_off, &site1_bytes),
        (site2.adrp_off, &adrp2),
        (site2.add_off, &add2),
        (cave.fd35_off, FD35_BYTES),
        (cave.fmt_off, FMT_UID_ONLY),
    ];
    edits.sort_by_key(|(off, _)| *off);
    for pair in edits.windows(2) {
        if pair[0].0 + pair[0].1.len() > pair[1].0 {
            return Err("adaptive UDS patch regions overlap");
        }
    }

    let mut changed = 0usize;
    for (off, after) in edits {
        let before = &bytes[off..off + after.len()];
        changed += before
            .iter()
            .zip(after.iter())
            .filter(|(a, b)| a != b)
            .count();
        bytes[off..off + after.len()].copy_from_slice(after);
    }
    if changed == 0 {
        return Err("adaptive UDS patch made no modifications");
    }
    Ok(changed)
}

#[cfg_attr(not(test), allow(dead_code))]
pub(super) fn apply_adaptive_99b(bytes: &mut [u8]) -> Result<usize, &'static str> {
    let fd_changed = crate::apply_fd_path_remaps(bytes).map_err(|_| {
        "runtime does not contain expected upstream /etc/resolv.conf or /etc/codex paths"
    })?;
    let uds_changed = apply_adaptive_uds_fd35(bytes)?;
    Ok(fd_changed + uds_changed)
}

#[cfg(test)]
pub(super) mod tests {
    use super::*;

    pub(crate) fn build_canonical_synthetic_aarch64_elf() -> Vec<u8> {
        build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical).0
    }

    #[derive(Clone, Copy)]
    enum Site1Variant {
        Canonical,
        RelocatedCaveAndReordered {
            mov_reg: u8,
            sp_imm: u16,
            interleaved: Option<u32>,
        },
        DuplicateSite1,
    }

    fn write_u16(buf: &mut [u8], off: usize, v: u16) {
        buf[off..off + 2].copy_from_slice(&v.to_le_bytes());
    }

    fn write_u32(buf: &mut [u8], off: usize, v: u32) {
        buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
    }

    fn write_u64(buf: &mut [u8], off: usize, v: u64) {
        buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
    }

    fn build_synthetic_aarch64_elf(
        rx_va_base: u64,
        occupy_cave_head: bool,
        occupy_entire_cave: bool,
        variant: Site1Variant,
    ) -> (Vec<u8>, usize, usize, usize) {
        // Layout:
        // 0x0000..0x0040 ELF header (64B)
        // 0x0040..0x0078 1 PT_LOAD RX phdr (56B)
        // 0x1000..0x1400 .text (1024B)
        // 0x1400..0x1410 .fini (16B)
        // 0x1420..0x2000 alignment cave between .fini and .rodata (all zero unless occupied)
        // 0x2000..0x2200 .rodata (daemon fmt string + upstream FD literal strings)
        // 0x2200..0x2260 .shstrtab
        // 0x2280..0x23c0 5 Shdrs (null, .text, .fini, .rodata, .shstrtab)
        let text_off = 0x1000usize;
        let text_sz = 0x400usize;
        let fini_off = 0x1400usize;
        let fini_sz = 0x10usize;
        let ro_off = 0x2000usize;
        let ro_sz = 0x200usize;
        let shstr_off = 0x2200usize;
        let shstr = b"\0.text\0.fini\0.rodata\0.shstrtab\0";
        let sh_off = 0x2280usize;
        let total = 0x2400usize;
        let mut buf = vec![0u8; total];

        buf[..4].copy_from_slice(b"\x7fELF");
        buf[4] = 2; // ELF64
        buf[5] = 1; // LE
        buf[6] = 1; // EV_CURRENT
        write_u16(&mut buf, 16, 2); // ET_EXEC
        write_u16(&mut buf, 18, 183); // EM_AARCH64
        write_u32(&mut buf, 20, 1); // EV_CURRENT
        write_u64(&mut buf, 32, 64); // e_phoff
        write_u64(&mut buf, 40, sh_off as u64); // e_shoff
        write_u16(&mut buf, 52, 64); // e_ehsize
        write_u16(&mut buf, 54, 56); // e_phentsize
        write_u16(&mut buf, 56, 1); // e_phnum
        write_u16(&mut buf, 58, 64); // e_shentsize
        write_u16(&mut buf, 60, 5); // e_shnum
        write_u16(&mut buf, 62, 4); // e_shstrndx

        // PT_LOAD covering full image as RX (flags = 5)
        let ph = 64usize;
        write_u32(&mut buf, ph, 1); // PT_LOAD
        write_u32(&mut buf, ph + 4, 5); // PF_R | PF_X
        write_u64(&mut buf, ph + 8, 0);
        write_u64(&mut buf, ph + 16, rx_va_base);
        write_u64(&mut buf, ph + 24, rx_va_base);
        write_u64(&mut buf, ph + 32, total as u64);
        write_u64(&mut buf, ph + 40, total as u64);
        write_u64(&mut buf, ph + 48, 0x1000);

        // .rodata strings
        let fmt_str_off = ro_off + 0x10;
        buf[fmt_str_off..fmt_str_off + DAEMON_FMT_NEEDLE.len()].copy_from_slice(DAEMON_FMT_NEEDLE);
        let mut cur = ro_off + 0x40;
        for lit in [
            b"/etc/resolv.conf".as_slice(),
            b"/etc/resolv.conf".as_slice(),
            b"/etc/codex/config.toml".as_slice(),
            b"/etc/codex/requirements.toml".as_slice(),
            b"/etc/codex/managed_config.toml".as_slice(),
        ] {
            buf[cur..cur + lit.len()].copy_from_slice(lit);
            cur += lit.len() + 1;
        }

        buf[shstr_off..shstr_off + shstr.len()].copy_from_slice(shstr);
        let mut write_shdr = |idx: usize, name_off: u32, sh_type: u32, off: usize, sz: usize| {
            let s = sh_off + idx * 64;
            write_u32(&mut buf, s, name_off);
            write_u32(&mut buf, s + 4, sh_type);
            write_u64(&mut buf, s + 24, off as u64);
            write_u64(&mut buf, s + 32, sz as u64);
        };
        write_shdr(1, 1, 1, text_off, text_sz);
        write_shdr(2, 7, 1, fini_off, fini_sz);
        write_shdr(3, 13, 1, ro_off, ro_sz);
        write_shdr(4, 21, 3, shstr_off, shstr.len());

        // Cave occupancy
        if occupy_cave_head {
            buf[fini_off + fini_sz + 0x10..fini_off + fini_sz + 0x30].fill(0x5a);
        }
        if occupy_entire_cave {
            buf[fini_off + fini_sz..ro_off].fill(0x7f);
        }

        // .text site1 + site2
        let site1_off = text_off + 0x80;
        let emit_site1 = |b: &mut [u8],
                          at: usize,
                          mov_reg: u8,
                          sp_imm: u16,
                          extra: Option<u32>,
                          permute: bool| {
            let movz = 0x528e_85e0 | u32::from(mov_reg);
            let movk = 0x72ae_0da0 | u32::from(mov_reg);
            let add_x0 = 0x9100_03e0 | (u32::from(sp_imm) << 10);
            let mov_w1 = 0x5280_00a1;
            let str_w = 0xb900_03e0 | (u32::from(sp_imm >> 2) << 10) | u32::from(mov_reg);
            let strb_wzr = 0x3900_03ff | (u32::from(sp_imm + 4) << 10);
            let seq = if let Some(mid) = extra {
                if permute {
                    vec![mov_w1, movz, mid, add_x0, movk, strb_wzr, str_w]
                } else {
                    vec![movz, add_x0, movk, mov_w1, strb_wzr, str_w, mid]
                }
            } else {
                vec![movz, add_x0, movk, mov_w1, strb_wzr, str_w]
            };
            for (idx, insn) in seq.into_iter().enumerate() {
                write_u32(b, at + idx * 4, insn);
            }
        };
        match variant {
            Site1Variant::Canonical => {
                emit_site1(&mut buf, site1_off, 8, 0x18, None, false);
            }
            Site1Variant::RelocatedCaveAndReordered {
                mov_reg,
                sp_imm,
                interleaved,
            } => {
                emit_site1(&mut buf, site1_off, mov_reg, sp_imm, interleaved, true);
            }
            Site1Variant::DuplicateSite1 => {
                emit_site1(&mut buf, site1_off, 8, 0x18, None, false);
                emit_site1(&mut buf, site1_off + 0x40, 9, 0x20, None, false);
            }
        }

        let site2_off = text_off + 0x140;
        let site2_va = rx_va_base + site2_off as u64;
        let fmt_va = rx_va_base + fmt_str_off as u64;
        write_u32(
            &mut buf,
            site2_off,
            encode_adrp(0, site2_va, fmt_va).unwrap(),
        );
        write_u32(
            &mut buf,
            site2_off + 4,
            encode_add_imm12(0, 0, (fmt_va & 0xfff) as u16).unwrap(),
        );
        (buf, site1_off, site2_off, fini_off + fini_sz)
    }

    #[test]
    fn runtime_artifact_patch_is_exact_bounded_and_rejects_drift() {
        assert_eq!(required("0.159.2"), Ok(None));
        for future in [
            "0.160.1", "0.161.1", "0.162.0", "0.163.1", "0.164.1", "invalid",
        ] {
            assert!(required(future).is_err());
        }
        for adaptive in ["0.165.0", "0.167.2", "0.169.0", "1.0.0"] {
            let p = required(adaptive).unwrap().unwrap();
            assert_eq!(p.id, ADAPTIVE_POLICY);
            assert!(supports_version(ADAPTIVE_POLICY, adaptive));
        }
        assert!(!supports_version(ADAPTIVE_POLICY, "0.164.0"));
        assert!(!supports_version(ADAPTIVE_POLICY, "0.162.0"));
        assert!(!supports_version(ADAPTIVE_POLICY, "0.159.2"));

        for version in ["0.160.0", "0.161.0", "0.163.0", "0.164.0"] {
            let policy = required(version).unwrap().unwrap();
            let raw_sha = policy.raw_sha256.unwrap();
            let expected_changed = policy.changed_bytes.unwrap();
            assert_eq!(policy.version, version);
            let (guard, patches) = blocks(policy);
            let size = patches
                .iter()
                .map(|(off, old, _)| off + old.len())
                .max()
                .unwrap()
                .max(guard + 16);
            let mut bytes = vec![0; size];
            bytes[guard..guard + 16].copy_from_slice(DAEMON_FMT_NEEDLE);
            for (off, old, _) in patches {
                bytes[*off..off + old.len()].copy_from_slice(old);
            }
            assert!(apply(&mut bytes, "unqualified", policy).is_err());
            bytes[guard] ^= 1;
            assert!(apply(&mut bytes, raw_sha, policy).is_err());
            bytes[guard] ^= 1;
            // Every block and truncated boundary refuses before any mutation.
            for (off, old, _) in patches {
                bytes[*off] ^= 1;
                assert!(
                    apply(&mut bytes, raw_sha, policy).is_err(),
                    "{version} drift at {off:x}"
                );
                bytes[*off] ^= 1;
                for (check, expected, _) in patches {
                    assert_eq!(&bytes[*check..check + expected.len()], *expected);
                }
                assert!(apply(&mut bytes[..off + old.len() - 1], raw_sha, policy).is_err());
            }
            let mut ranges = patches
                .iter()
                .map(|(off, old, _)| *off..off + old.len())
                .collect::<Vec<_>>();
            ranges.sort_by_key(|range| range.start);
            assert!(ranges.windows(2).all(|pair| pair[0].end <= pair[1].start));
            let before = bytes.clone();
            assert_eq!(
                apply(&mut bytes, raw_sha, policy).unwrap(),
                expected_changed
            );
            assert_eq!(before.len(), bytes.len());
            for (index, (a, b)) in before.iter().zip(&bytes).enumerate() {
                if a != b {
                    assert!(patches
                        .iter()
                        .any(|(off, old, _)| (*off..off + old.len()).contains(&index)));
                }
            }
            assert!(apply(&mut bytes, raw_sha, policy).is_err());
        }
    }

    #[test]
    fn adaptive_99b_handles_relocation_reordering_va_shifts_and_rejects_invalid() {
        let policy = required("0.169.0").unwrap().unwrap();
        let (mut canonical, site1_off, site2_off, fini_end) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        let fd_changed = crate::apply_fd_path_remaps(&mut canonical).unwrap();
        let uds_changed = apply(&mut canonical, "any-digest", policy).unwrap();
        assert_eq!(fd_changed, 54);
        assert!(uds_changed > 0);
        let cave0_fd35 = (fini_end + 16 + 15) & !15;
        assert_eq!(
            &canonical[cave0_fd35..cave0_fd35 + FD35_BYTES.len()],
            FD35_BYTES
        );
        assert_eq!(
            &canonical[cave0_fd35 + 0x20..cave0_fd35 + 0x22],
            FMT_UID_ONLY
        );
        assert_eq!(
            le_u32(&canonical, site1_off + 8),
            Some(encode_movz_w(1, FD35_BYTES.len() as u16))
        );
        // Re-applying fails closed after site2 and site1 are already patched.
        assert!(apply(&mut canonical, "any-digest", policy).is_err());
        assert_ne!(site2_off, 0);

        // Relocated cave + reordered site1 with safe interleaved `sub x8, x29, #0x18` + shifted VA base.
        let sub_x8_x29_18 = 0xd100_63a8u32;
        let (mut shifted, sh_site1, _, _) = build_synthetic_aarch64_elf(
            0x410_000,
            true,
            false,
            Site1Variant::RelocatedCaveAndReordered {
                mov_reg: 19,
                sp_imm: 0x40,
                interleaved: Some(sub_x8_x29_18),
            },
        );
        let before_shifted = shifted.clone();
        let shifted_changed = apply_adaptive_99b(&mut shifted).unwrap();
        let actual_shifted_diffs = before_shifted
            .iter()
            .zip(shifted.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(shifted_changed, actual_shifted_diffs);
        assert!(shifted_changed > fd_changed);
        assert_eq!(le_u32(&shifted, sh_site1 + 12), Some(sub_x8_x29_18));
        assert_eq!(le_u32(&shifted, sh_site1 + 16), Some(NOP_INSN));
        assert_eq!(le_u32(&shifted, sh_site1 + 20), Some(NOP_INSN));
        assert_eq!(le_u32(&shifted, sh_site1 + 24), Some(NOP_INSN));
        let moved_cave = cave0_fd35 + 0x20;
        assert_eq!(
            &shifted[moved_cave..moved_cave + FD35_BYTES.len()],
            FD35_BYTES
        );
        assert_eq!(&shifted[moved_cave + 0x20..moved_cave + 0x22], FMT_UID_ONLY);
        // Negative cases fail closed before mutation:
        let (mut no_cave, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, true, Site1Variant::Canonical);
        assert!(apply_adaptive_uds_fd35(&mut no_cave).is_err());

        let (mut with_interp, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        write_u32(&mut with_interp, 64, 3); // PT_INTERP in phdr0
        assert!(apply_adaptive_uds_fd35(&mut with_interp).is_err());

        let (mut bad_arch, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        write_u16(&mut bad_arch, 18, 62); // x86_64
        assert!(apply_adaptive_uds_fd35(&mut bad_arch).is_err());

        let (mut dup_site1, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::DuplicateSite1);
        assert!(apply_adaptive_uds_fd35(&mut dup_site1).is_err());

        // Unsafe interleaved instruction touching X0 (`add x0, x19, #8` = 0x91002260) must reject.
        let (mut unsafe_interleaved, _, _, _) = build_synthetic_aarch64_elf(
            0x400_000,
            false,
            false,
            Site1Variant::RelocatedCaveAndReordered {
                mov_reg: 9,
                sp_imm: 0x20,
                interleaved: Some(0x9100_2260),
            },
        );
        assert!(apply_adaptive_uds_fd35(&mut unsafe_interleaved).is_err());
    }
}
