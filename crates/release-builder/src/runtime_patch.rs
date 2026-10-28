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

// Publication metadata is shared by adaptation, descriptor writing and admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Policy {
    pub id: &'static str,
    pub version: &'static str,
    pub raw_sha256: &'static str,
    pub archive_sha256: Option<&'static str>,
    pub uds: &'static str,
    pub permission: Option<&'static str>,
    pub changed_bytes: usize,
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
            raw_sha256: RAW_SHA256,
            archive_sha256: None,
            uds: "termux-uds-0-160-0-v1",
            permission: (id == POLICY).then_some(PERMISSION_POLICY),
            changed_bytes: if id == POLICY { CHANGED_BYTES } else { 43 },
        }),
        "termux-fd-remap-v4" => Some(Policy {
            id: "termux-fd-remap-v4",
            version: "0.161.0",
            raw_sha256: "0129f94f4f1197bd75f6c4265889061f386ed302bbc50c6e57e3e0660dfb28a7",
            archive_sha256: Some(
                "3c02e2ae34be0d06e62557e98fc5c0a783bec5a2fed406fe00e565803bf84ee8",
            ),
            uds: "termux-uds-0-161-0-v1",
            permission: Some("termux-permission-picker-0-161-0-v1"),
            changed_bytes: 432,
        }),
        "termux-fd-remap-v5" => Some(Policy {
            id: "termux-fd-remap-v5",
            version: "0.163.0",
            raw_sha256: "8a96f66d059d653e68bfd5f3c79337e6f1d7d170d80bf705eb7d0249b34c0ece",
            archive_sha256: Some(
                "9d8ecf2d19d3e184877983902d6c85de48b3888dcb72c382a8a0c975845927fb",
            ),
            uds: "termux-uds-0-163-0-v1",
            permission: Some("termux-permission-picker-0-163-0-v1"),
            changed_bytes: 433,
        }),
        "termux-fd-remap-v6" => Some(Policy {
            id: "termux-fd-remap-v6",
            version: "0.164.0",
            raw_sha256: "8813c8d8a4cea2eeba271b72ab22f49628b184b24ab18dbef06b4ce41e9a8d39",
            archive_sha256: Some(
                "5918add6c7e9cf1e14a9af1b4c0f4bf1d24a43903faac26df5dc4c019ea91bd5",
            ),
            uds: "termux-uds-0-164-0-v1",
            permission: Some("termux-permission-picker-0-164-0-v1"),
            changed_bytes: 478,
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
        older if older < [0, 160, 0].as_slice() => Ok(None),
        _ => Err("upstream UDS version requires fresh Termux qualification"),
    }
}

pub(super) fn supports_version(id: &str, version: &str) -> bool {
    if id == super::PATCH_POLICY_ID {
        required(version).is_ok_and(|policy| policy.is_none_or(|policy| policy.id == POLICY))
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
    let (guard, patches) = blocks(policy);
    if policy.permission.is_none()
        || raw_digest != policy.raw_sha256
        || bytes.get(guard..guard + 16) != Some(b"\x0dcodex-daemon-\xc0\0")
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
    if changed != policy.changed_bytes {
        return Err("qualified Termux changed-byte count is inconsistent");
    }
    for (off, _, new) in patches {
        bytes[*off..off + new.len()].copy_from_slice(new);
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn runtime_artifact_patch_is_exact_bounded_and_rejects_drift() {
        assert_eq!(required("0.159.2"), Ok(None));
        for future in [
            "0.160.1", "0.161.1", "0.162.0", "0.163.1", "0.164.1", "0.165.0", "1.0.0", "invalid",
        ] {
            assert!(required(future).is_err());
        }
        for version in ["0.160.0", "0.161.0", "0.163.0", "0.164.0"] {
            let policy = required(version).unwrap().unwrap();
            assert_eq!(policy.version, version);
            let (guard, patches) = blocks(policy);
            let size = patches
                .iter()
                .map(|(off, old, _)| off + old.len())
                .max()
                .unwrap()
                .max(guard + 16);
            let mut bytes = vec![0; size];
            bytes[guard..guard + 16].copy_from_slice(b"\x0dcodex-daemon-\xc0\0");
            for (off, old, _) in patches {
                bytes[*off..off + old.len()].copy_from_slice(old);
            }
            assert!(apply(&mut bytes, "unqualified", policy).is_err());
            bytes[guard] ^= 1;
            assert!(apply(&mut bytes, policy.raw_sha256, policy).is_err());
            bytes[guard] ^= 1;
            // Every block and truncated boundary refuses before any mutation.
            for (off, old, _) in patches {
                bytes[*off] ^= 1;
                assert!(
                    apply(&mut bytes, policy.raw_sha256, policy).is_err(),
                    "{version} drift at {off:x}"
                );
                bytes[*off] ^= 1;
                for (check, expected, _) in patches {
                    assert_eq!(&bytes[*check..check + expected.len()], *expected);
                }
                assert!(
                    apply(&mut bytes[..off + old.len() - 1], policy.raw_sha256, policy).is_err()
                );
            }
            let mut ranges = patches
                .iter()
                .map(|(off, old, _)| *off..off + old.len())
                .collect::<Vec<_>>();
            ranges.sort_by_key(|range| range.start);
            assert!(ranges.windows(2).all(|pair| pair[0].end <= pair[1].start));
            let before = bytes.clone();
            assert_eq!(
                apply(&mut bytes, policy.raw_sha256, policy).unwrap(),
                policy.changed_bytes
            );
            assert_eq!(before.len(), bytes.len());
            for (index, (a, b)) in before.iter().zip(&bytes).enumerate() {
                if a != b {
                    assert!(patches
                        .iter()
                        .any(|(off, old, _)| (*off..off + old.len()).contains(&index)));
                }
            }
            assert!(apply(&mut bytes, policy.raw_sha256, policy).is_err());
        }
    }
}
