//! Rule-based Termux adaptation of the protected UDS root.
pub(super) const PATCH_POLICY_ID: &str = "termux-fd-remap";
pub(super) const UDS_POLICY_ID: &str = "termux-uds-fd35";
const FD35_BYTES: &[u8] = b"/proc/self/fd/35\0";
const FMT_UID_ONLY: &[u8] = b"\xc0\0";
const DAEMON_FMT_NEEDLE: &[u8] = b"\x0dcodex-daemon-\xc0\0";
const NOP_INSN: u32 = 0xd503_201f;
const FD35_CAVE_PREFIX: usize = 0x30;
const REALPATH_STUB_MAX_BYTES: usize = 0x140;
const RX_CAVE_MIN_BYTES: usize = FD35_CAVE_PREFIX + REALPATH_STUB_MAX_BYTES;

pub(super) fn contains_daemon_fmt(bytes: &[u8]) -> bool {
    // Detection must not depend on the layout parser used to apply the patch.
    // Otherwise section/layout drift silently admits an unadapted UDS runtime.
    bytes
        .windows(DAEMON_FMT_NEEDLE.len())
        .any(|w| w == DAEMON_FMT_NEEDLE)
}

#[derive(Clone, Copy, Debug)]
struct ElfSection {
    off: usize,
    sz: usize,
}

#[derive(Clone, Copy, Debug)]
struct ElfLoad {
    phdr_off: usize,
    flags: u32,
    off: usize,
    va: u64,
    filesz: usize,
    memsz: usize,
    align: u64,
    delta: i128,
}

#[derive(Clone, Debug)]
struct ParsedElf {
    fini: ElfSection,
    rodata: ElfSection,
    text: ElfSection,
    rx_load: ElfLoad,
    rx_sections: Vec<ElfSection>,
    internal_tail_end: usize,
    max_rx_extend_end: usize,
    ro_delta: i128,
}

#[derive(Clone, Copy, Debug)]
struct RxCave {
    fd35_off: usize,
    fmt_off: usize,
    stub_off: usize,
    stub_cap: usize,
    extend_rx_to: Option<usize>,
}

#[derive(Clone, Copy, Debug)]
struct RealpathMatch {
    realpath_off: usize,
    orig_insn0: u32,
    strdup_off: usize,
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

fn va_to_off(va: u64, seg: ElfLoad) -> Result<usize, &'static str> {
    let rel = i128::from(va) - seg.delta;
    let off = usize::try_from(rel).map_err(|_| "ELF virtual address precedes segment")?;
    if off < seg.off || off >= seg.off + seg.filesz {
        return Err("ELF virtual address outside RX segment");
    }
    Ok(off)
}

fn decode_b_or_bl_target(insn: u32, pc_va: u64) -> Option<(bool, u64)> {
    if (insn & 0x7c00_0000) != 0x1400_0000 {
        return None;
    }
    let is_link = (insn >> 31) != 0;
    let mut imm26 = (insn & 0x03ff_ffff) as i32;
    if (imm26 & (1 << 25)) != 0 {
        imm26 -= 1 << 26;
    }
    let target = i128::from(pc_va) + i128::from(imm26) * 4;
    let va = u64::try_from(target).ok()?;
    Some((is_link, va))
}

fn decode_cond_or_cbz_target(insn: u32, pc_va: u64) -> Option<u64> {
    let diff = if (insn & 0xff00_0010) == 0x5400_0000 || (insn & 0x7e00_0000) == 0x3400_0000 {
        let mut imm19 = ((insn >> 5) & 0x7_ffff) as i32;
        if (imm19 & (1 << 18)) != 0 {
            imm19 -= 1 << 19;
        }
        i128::from(imm19) * 4
    } else if (insn & 0x7e00_0000) == 0x3600_0000 {
        let mut imm14 = ((insn >> 5) & 0x3fff) as i32;
        if (imm14 & (1 << 13)) != 0 {
            imm14 -= 1 << 14;
        }
        i128::from(imm14) * 4
    } else {
        return None;
    };
    u64::try_from(i128::from(pc_va) + diff).ok()
}

fn branch_target_va(insn: u32, pc_va: u64) -> Option<u64> {
    decode_b_or_bl_target(insn, pc_va)
        .map(|(_, va)| va)
        .or_else(|| decode_cond_or_cbz_target(insn, pc_va))
}

fn encode_b(pc_va: u64, target_va: u64, link: bool) -> Result<u32, &'static str> {
    let diff = (i128::from(target_va) - i128::from(pc_va)) >> 2;
    if !(-(1 << 25)..(1 << 25)).contains(&diff) {
        return Err("AArch64 B/BL target out of +/-128MiB range");
    }
    let op = if link { 0x9400_0000 } else { 0x1400_0000 };
    Ok(op | ((diff as u32) & 0x03ff_ffff))
}

fn encode_b_cond(pc_va: u64, target_va: u64, cond: u32) -> Result<u32, &'static str> {
    let diff = (i128::from(target_va) - i128::from(pc_va)) >> 2;
    if !(-(1 << 18)..(1 << 18)).contains(&diff) {
        return Err("AArch64 B.cond target out of range");
    }
    Ok(0x5400_0000 | (((diff as u32) & 0x7_ffff) << 5) | (cond & 0xf))
}

fn encode_cbz(
    pc_va: u64,
    target_va: u64,
    rt: u8,
    nz: bool,
    is64: bool,
) -> Result<u32, &'static str> {
    let diff = (i128::from(target_va) - i128::from(pc_va)) >> 2;
    if !(-(1 << 18)..(1 << 18)).contains(&diff) {
        return Err("AArch64 CBZ/CBNZ target out of range");
    }
    let op = match (is64, nz) {
        (true, true) => 0xb500_0000,
        (true, false) => 0xb400_0000,
        (false, true) => 0x3500_0000,
        (false, false) => 0x3400_0000,
    };
    Ok(op | (((diff as u32) & 0x7_ffff) << 5) | u32::from(rt & 0x1f))
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
    let mut non_load_min_after = Vec::new();
    for i in 0..e_phnum {
        let ph = e_phoff + i * 56;
        let p_type = le_u32(data, ph).ok_or("corrupt ELF phdr")?;
        if p_type == 3 {
            return Err("executable has PT_INTERP");
        }
        let off = usize::try_from(le_u64(data, ph + 8).ok_or("corrupt ELF phdr")?)
            .map_err(|_| "ELF phdr offset overflow")?;
        let filesz = usize::try_from(le_u64(data, ph + 32).ok_or("corrupt ELF phdr")?)
            .map_err(|_| "ELF phdr size overflow")?;
        if p_type == 1 {
            let flags = le_u32(data, ph + 4).ok_or("corrupt ELF phdr")?;
            let va = le_u64(data, ph + 16).ok_or("corrupt ELF phdr")?;
            let memsz = usize::try_from(le_u64(data, ph + 40).ok_or("corrupt ELF phdr")?)
                .map_err(|_| "ELF phdr memsz overflow")?;
            let align = le_u64(data, ph + 48).ok_or("corrupt ELF phdr")?;
            if filesz > memsz || off.checked_add(filesz).is_none_or(|end| end > data.len()) {
                return Err("ELF PT_LOAD exceeds file bounds");
            }
            loads.push(ElfLoad {
                phdr_off: ph,
                flags,
                off,
                va,
                filesz,
                memsz,
                align,
                delta: i128::from(va) - off as i128,
            });
        } else if filesz > 0 {
            non_load_min_after.push(off);
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
    let mut alloc_or_rx_sections = Vec::new();
    let mut file_section_starts = Vec::new();
    for i in 0..e_shnum {
        let sh = e_shoff + i * e_shentsize;
        let sh_name = usize::try_from(le_u32(data, sh).ok_or("corrupt shdr")?)
            .map_err(|_| "section name overflow")?;
        let sh_type = le_u32(data, sh + 4).ok_or("corrupt shdr")?;
        let sh_flags = le_u64(data, sh + 8).ok_or("corrupt shdr")?;
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
        if sh_type != 8 && sz > 0 {
            file_section_starts.push(off);
        }
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
        if sh_type != 8
            && sz > 0
            && ((sh_flags & 2) != 0 || matches!(name, b".init" | b".text" | b".fini" | b".rodata"))
        {
            alloc_or_rx_sections.push(ElfSection { off, sz });
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
    let rx_load = rx_matches[0];
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

    let rx_end = rx_load
        .off
        .checked_add(rx_load.filesz)
        .ok_or("ELF RX PT_LOAD overflow")?;
    let mut rx_sections: Vec<ElfSection> = alloc_or_rx_sections
        .into_iter()
        .filter(|s| {
            s.off >= rx_load.off && s.off.checked_add(s.sz).is_some_and(|end| end <= rx_end)
        })
        .collect();
    rx_sections.sort_by_key(|s| (s.off, s.sz));
    let rx_last_end = rx_sections
        .iter()
        .map(|s| s.off + s.sz)
        .max()
        .unwrap_or(rx_load.off);

    let mut internal_tail_end = rx_end;
    if e_phoff >= rx_last_end {
        internal_tail_end = internal_tail_end.min(e_phoff);
    }
    if e_shoff >= rx_last_end {
        internal_tail_end = internal_tail_end.min(e_shoff);
    }
    if rodata.off >= rx_last_end {
        internal_tail_end = internal_tail_end.min(rodata.off);
    }
    for &off in file_section_starts.iter().chain(non_load_min_after.iter()) {
        if off >= rx_last_end {
            internal_tail_end = internal_tail_end.min(off);
        }
    }

    let max_rx_extend_end = if rx_load.filesz != rx_load.memsz {
        rx_end
    } else {
        let mut limit = data.len();
        if e_phoff >= rx_end {
            limit = limit.min(e_phoff);
        }
        if e_shoff >= rx_end {
            limit = limit.min(e_shoff);
        }
        for off in file_section_starts.into_iter().chain(non_load_min_after) {
            if off >= rx_end {
                limit = limit.min(off);
            }
        }
        for l in &loads {
            if l.phdr_off == rx_load.phdr_off {
                continue;
            }
            if l.off >= rx_end {
                limit = limit.min(l.off);
            }
            if l.va > rx_load.va {
                let page_align = if l.align >= 0x1000 && l.align.is_power_of_two() {
                    l.align.min(0x10000)
                } else {
                    0x1000
                };
                let page_va = l.va & !(page_align - 1);
                if let Ok(va_limit_off) = usize::try_from(i128::from(page_va) - rx_load.delta) {
                    limit = limit.min(va_limit_off);
                }
            }
        }
        limit.max(rx_end)
    };

    Ok(ParsedElf {
        text,
        fini,
        rodata,
        rx_load,
        rx_sections,
        internal_tail_end,
        max_rx_extend_end,
        ro_delta: ro_matches[0].delta,
    })
}

fn find_zero_slot(
    data: &[u8],
    gap_start: usize,
    gap_end: usize,
    needed: usize,
    require_contiguous_zero_prefix: bool,
) -> Option<usize> {
    let mut cand = (gap_start.checked_add(16)? + 15) & !15;
    if require_contiguous_zero_prefix
        && data
            .get(gap_start..cand)
            .is_none_or(|prefix| prefix.iter().any(|&b| b != 0))
    {
        return None;
    }
    while cand.checked_add(needed).is_some_and(|end| end <= gap_end) {
        if data[cand..cand + needed].iter().all(|&b| b == 0) {
            return Some(cand);
        }
        if require_contiguous_zero_prefix {
            break;
        }
        cand += 16;
    }
    None
}

fn locate_rx_cave(
    data: &[u8],
    elf: &ParsedElf,
    realpath_off: usize,
) -> Result<RxCave, &'static str> {
    let fini_end = elf
        .fini
        .off
        .checked_add(elf.fini.sz)
        .ok_or("ELF .fini overflow")?;
    let rx_end = elf.rx_load.off + elf.rx_load.filesz;
    let realpath_va = off_to_va(realpath_off, elf.rx_load.delta)?;

    let mut internal_gaps: Vec<(usize, usize, bool)> = Vec::new();
    if let Some(first) = elf.rx_sections.first() {
        let mut prev_end = first.off + first.sz;
        for sec in &elf.rx_sections[1..] {
            if sec.off > prev_end {
                let is_fini_rodata = prev_end == fini_end && sec.off == elf.rodata.off;
                internal_gaps.push((prev_end, sec.off, is_fini_rodata));
            }
            prev_end = prev_end.max(sec.off + sec.sz);
        }
        if elf.internal_tail_end > prev_end {
            let is_fini_rodata = prev_end == fini_end && elf.rodata.off >= fini_end;
            internal_gaps.push((prev_end, elf.internal_tail_end, is_fini_rodata));
        }
    }
    internal_gaps
        .sort_by_key(|&(start, _, is_fini_rodata)| (!is_fini_rodata, start.abs_diff(realpath_off)));

    // Pass 1: single contiguous internal RX alignment gap (no PHDR mutation).
    for &(g_start, g_end, _) in &internal_gaps {
        if let Some(cand) = find_zero_slot(data, g_start, g_end, RX_CAVE_MIN_BYTES, false) {
            let stub_off = cand + FD35_CAVE_PREFIX;
            let stub_va = off_to_va(stub_off, elf.rx_load.delta)?;
            if encode_b(realpath_va, stub_va, false).is_ok() {
                return Ok(RxCave {
                    fd35_off: cand,
                    fmt_off: cand + 0x20,
                    stub_off,
                    stub_cap: REALPATH_STUB_MAX_BYTES,
                    extend_rx_to: None,
                });
            }
        }
    }

    // Pass 2: contiguous all-zero padding immediately following RX PT_LOAD (extends p_filesz/p_memsz).
    if elf.max_rx_extend_end > rx_end {
        if let Some(cand) =
            find_zero_slot(data, rx_end, elf.max_rx_extend_end, RX_CAVE_MIN_BYTES, true)
        {
            let stub_off = cand + FD35_CAVE_PREFIX;
            let stub_va = off_to_va(stub_off, elf.rx_load.delta)?;
            if encode_b(realpath_va, stub_va, false).is_ok() {
                return Ok(RxCave {
                    fd35_off: cand,
                    fmt_off: cand + 0x20,
                    stub_off,
                    stub_cap: REALPATH_STUB_MAX_BYTES,
                    extend_rx_to: Some(cand + RX_CAVE_MIN_BYTES),
                });
            }
        }
    }

    // Pass 3: split strings (0x30B) and stub (0x140B) across two separate RX gaps/extension.
    let mut string_slots: Vec<(usize, Option<usize>)> = Vec::new();
    for &(g_start, g_end, _) in &internal_gaps {
        if let Some(cand) = find_zero_slot(data, g_start, g_end, FD35_CAVE_PREFIX, false) {
            string_slots.push((cand, None));
        }
    }
    if elf.max_rx_extend_end > rx_end {
        if let Some(cand) =
            find_zero_slot(data, rx_end, elf.max_rx_extend_end, FD35_CAVE_PREFIX, true)
        {
            string_slots.push((cand, Some(cand + FD35_CAVE_PREFIX)));
        }
    }

    let mut stub_slots: Vec<(usize, Option<usize>)> = Vec::new();
    for &(g_start, g_end, _) in &internal_gaps {
        if let Some(cand) = find_zero_slot(data, g_start, g_end, REALPATH_STUB_MAX_BYTES, false) {
            stub_slots.push((cand, None));
        }
    }
    if elf.max_rx_extend_end > rx_end {
        if let Some(cand) = find_zero_slot(
            data,
            rx_end,
            elf.max_rx_extend_end,
            REALPATH_STUB_MAX_BYTES,
            true,
        ) {
            stub_slots.push((cand, Some(cand + REALPATH_STUB_MAX_BYTES)));
        }
    }

    for &(str_off, str_ext) in &string_slots {
        for &(stub_off, stub_ext) in &stub_slots {
            let overlaps = !(str_off + FD35_CAVE_PREFIX <= stub_off
                || stub_off + REALPATH_STUB_MAX_BYTES <= str_off);
            if overlaps {
                continue;
            }
            let stub_va = off_to_va(stub_off, elf.rx_load.delta)?;
            if encode_b(realpath_va, stub_va, false).is_ok() {
                let extend_rx_to = match (str_ext, stub_ext) {
                    (Some(a), Some(b)) => Some(a.max(b)),
                    (Some(a), None) | (None, Some(a)) => Some(a),
                    (None, None) => None,
                };
                return Ok(RxCave {
                    fd35_off: str_off,
                    fmt_off: str_off + 0x20,
                    stub_off,
                    stub_cap: REALPATH_STUB_MAX_BYTES,
                    extend_rx_to,
                });
            }
        }
    }

    Err("insufficient all-zero RX alignment or trailing PT_LOAD cave for FD 35 hook")
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

fn decode_adrp_page(insn: u32, pc_va: u64, expected_rd: u8) -> Option<u64> {
    if (insn & 0x9f00_001f) != (0x9000_0000 | u32::from(expected_rd & 0x1f)) {
        return None;
    }
    let immlo = i128::from((insn >> 29) & 0x3);
    let immhi = i128::from((insn >> 5) & 0x7_ffff);
    let mut imm = (immhi << 2) | immlo;
    if (imm & (1 << 20)) != 0 {
        imm -= 1 << 21;
    }
    let pc_page = i128::from(pc_va & !0xfff);
    u64::try_from(pc_page + (imm << 12)).ok()
}

fn resolve_bl_target(data: &[u8], elf: &ParsedElf, bl_off: usize) -> Result<usize, &'static str> {
    let insn = le_u32(data, bl_off).ok_or("truncated BL instruction")?;
    let pc_va = off_to_va(bl_off, elf.rx_load.delta)?;
    let (is_link, target_va) =
        decode_b_or_bl_target(insn, pc_va).ok_or("expected BL instruction")?;
    if !is_link {
        return Err("expected BL instruction");
    }
    let target_off = va_to_off(target_va, elf.rx_load)?;
    if target_off + 12 <= elf.text.off + elf.text.sz {
        let w0 = le_u32(data, target_off).unwrap();
        let w1 = le_u32(data, target_off + 4).unwrap();
        let w2 = le_u32(data, target_off + 8).unwrap();
        if let Some(page) = decode_adrp_page(w0, target_va, 16) {
            if (w1 & 0xffc0_03ff) == 0x9100_0210 && w2 == 0xd61f_0200 {
                let lo12 = u64::from((w1 >> 10) & 0xfff);
                return va_to_off(page + lo12, elf.rx_load);
            }
        }
    }
    Ok(target_off)
}

fn is_memcpy_sequence_with_plus1(data: &[u8], start_off: usize, limit_off: usize) -> bool {
    let mut found_add_plus1 = false;
    let end = limit_off.min(start_off + 24);
    let mut off = start_off;
    while off + 4 <= end {
        let Some(w) = le_u32(data, off) else {
            return false;
        };
        if (w & 0xffff_fc1f) == 0x9100_0402 {
            found_add_plus1 = true;
        } else if let Some((is_link, _)) = decode_b_or_bl_target(w, (off as u64).wrapping_mul(1)) {
            return is_link && found_add_plus1;
        } else if is_branch_or_call(w) {
            return false;
        }
        off += 4;
    }
    false
}

fn extract_strdup_bl(
    data: &[u8],
    elf: &ParsedElf,
    block_off: usize,
    limit_off: usize,
) -> Option<usize> {
    let end = limit_off.min(block_off + 20);
    let mut saw_mov_x0 = false;
    let mut off = block_off;
    while off + 4 <= end {
        let w = le_u32(data, off)?;
        if (w & 0xffe0_ffff) == 0xaa00_03e0 {
            saw_mov_x0 = true;
        } else if let Some((true, _)) = decode_b_or_bl_target(w, 0) {
            if !saw_mov_x0 {
                return None;
            }
            let next_w = le_u32(data, off + 4)?;
            if (next_w & 0xfc00_0000) == 0x1400_0000 || next_w == 0xd65f_03c0 {
                return resolve_bl_target(data, elf, off).ok();
            }
            return None;
        } else if is_branch_or_call(w) {
            return None;
        }
        off += 4;
    }
    None
}

fn locate_realpath_and_strdup(
    data: &[u8],
    elf: &ParsedElf,
    site1_end: usize,
    site2_off: usize,
) -> Result<RealpathMatch, &'static str> {
    let text_end = elf.text.off + elf.text.sz;
    let search_end = site2_off.min(text_end);
    let mut rp_bl_offs = Vec::new();
    let mut off = site1_end;
    while off + 16 <= search_end {
        let w = le_u32(data, off).ok_or("truncated .text scan for realpath call")?;
        if matches!(w, 0xaa1f_03e1 | 0xd280_0001 | 0x5280_0001 | 0x2a1f_03e1) {
            for k in 1usize..=3 {
                let Some(bl_off) = off.checked_add(k * 4).filter(|&p| p + 8 <= search_end) else {
                    break;
                };
                let wk = le_u32(data, bl_off).unwrap();
                if let Some((true, _)) = decode_b_or_bl_target(wk, 0) {
                    let has_cbz_x0 = (1usize..=3).any(|m| {
                        bl_off
                            .checked_add(m * 4)
                            .and_then(|p| le_u32(data, p))
                            .is_some_and(|wm| (wm & 0xfe00_001f) == 0xb400_0000)
                    });
                    if has_cbz_x0 {
                        rp_bl_offs.push(bl_off);
                    }
                    break;
                }
                if is_branch_or_call(wk) {
                    break;
                }
            }
        }
        off += 4;
    }
    if rp_bl_offs.len() != 1 {
        return Err("expected unique realpath(..., NULL) call after /tmp UDS setup");
    }
    let realpath_off = resolve_bl_target(data, elf, rp_bl_offs[0])?;
    if realpath_off < elf.text.off || realpath_off + 0x40 > text_end {
        return Err("resolved realpath function falls outside .text bounds");
    }
    let orig_insn0 = le_u32(data, realpath_off).ok_or("truncated realpath prologue")?;
    if is_pc_relative(orig_insn0) || is_branch_or_call(orig_insn0) {
        return Err("realpath entry instruction is PC-relative or a branch");
    }

    let realpath_va = off_to_va(realpath_off, elf.rx_load.delta)?;
    let scan_end = text_end.min(realpath_off + 0x700);
    let mut saw_einval_cmp = false;
    let mut strdup_candidates = Vec::new();
    let mut pos = realpath_off + 4;
    while pos + 4 <= scan_end {
        let w = le_u32(data, pos).unwrap();
        let pc_va = off_to_va(pos, elf.rx_load.delta)?;
        if branch_target_va(w, pc_va) == Some(realpath_va) {
            return Err("realpath contains an internal branch to its entry instruction");
        }
        if (w & 0xffff_fc1f) == 0x7100_581f {
            saw_einval_cmp = true;
        }
        if (w & 0xff00_0000) == 0xb400_0000 {
            if let Some(target_va) = decode_cond_or_cbz_target(w, pc_va) {
                if let Ok(target_off) = va_to_off(target_va, elf.rx_load) {
                    if target_off >= realpath_off
                        && target_off < scan_end
                        && is_memcpy_sequence_with_plus1(data, pos + 4, scan_end)
                    {
                        if let Some(strdup_off) = extract_strdup_bl(data, elf, target_off, scan_end)
                        {
                            strdup_candidates.push(strdup_off);
                        }
                    }
                }
            }
        } else if (w & 0xff00_0000) == 0xb500_0000 {
            if let Some(target_va) = decode_cond_or_cbz_target(w, pc_va) {
                if let Ok(target_off) = va_to_off(target_va, elf.rx_load) {
                    if target_off >= realpath_off
                        && target_off < scan_end
                        && is_memcpy_sequence_with_plus1(data, target_off, scan_end)
                    {
                        if let Some(strdup_off) = extract_strdup_bl(data, elf, pos + 4, scan_end) {
                            strdup_candidates.push(strdup_off);
                        }
                    }
                }
            }
        }
        pos += 4;
    }

    if !saw_einval_cmp {
        return Err("resolved realpath candidate lacks EINVAL symlink termination check");
    }
    strdup_candidates.sort_unstable();
    strdup_candidates.dedup();
    if strdup_candidates.len() != 1 {
        return Err("expected unique strdup fallback inside realpath");
    }
    Ok(RealpathMatch {
        realpath_off,
        orig_insn0,
        strdup_off: strdup_candidates[0],
    })
}

fn build_realpath_stub(
    stub_va: u64,
    fd35_va: u64,
    realpath_va: u64,
    orig_insn0: u32,
    strdup_va: u64,
) -> Result<Vec<u8>, &'static str> {
    enum StubItem {
        Raw(u32),
        B(&'static str),
        BCond(&'static str, u32),
        Cbz(&'static str, u8, bool, bool),
        Adrp(u8, u64),
    }

    let mut items: Vec<StubItem> = Vec::with_capacity(80);
    let mut labels: std::collections::HashMap<&'static str, usize> =
        std::collections::HashMap::new();
    let label_va = |lbl: &'static str, map: &std::collections::HashMap<&'static str, usize>| {
        stub_va + (*map.get(lbl).expect("label defined") as u64) * 4
    };

    labels.insert("entry", items.len());
    items.push(StubItem::Raw(0xd108_83ff)); // sub sp, sp, #0x220
    items.push(StubItem::Raw(0xa900_7bfd)); // stp x29, x30, [sp]
    items.push(StubItem::Raw(0xa901_53f3)); // stp x19, x20, [sp, #0x10]
    items.push(StubItem::Cbz("call_orig_raw", 0, false, true));
    items.push(StubItem::Raw(0xaa00_03f3)); // mov x19, x0 (path)
    items.push(StubItem::Raw(0xaa01_03f4)); // mov x20, x1 (resolved)
    items.push(StubItem::Adrp(2, fd35_va));
    items.push(StubItem::Raw(encode_add_imm12(
        2,
        2,
        (fd35_va & 0xfff) as u16,
    )?));
    items.push(StubItem::Raw(0xaa13_03e3)); // mov x3, x19

    labels.insert("cmp1", items.len());
    items.push(StubItem::Raw(0x3840_1444)); // ldrb w4, [x2], #1
    items.push(StubItem::Cbz("cmp1_end", 4, false, false));
    items.push(StubItem::Raw(0x3840_1465)); // ldrb w5, [x3], #1
    items.push(StubItem::Raw(0x6b05_009f)); // cmp w4, w5
    items.push(StubItem::BCond("cmp1", 0));
    items.push(StubItem::B("try_readlink"));

    labels.insert("cmp1_end", items.len());
    items.push(StubItem::Raw(0x3940_0065)); // ldrb w5, [x3]
    items.push(StubItem::Cbz("use_x19", 5, false, false));
    items.push(StubItem::Raw(0x7100_bcbf)); // cmp w5, #0x2f ('/')
    items.push(StubItem::BCond("use_x19", 0));

    labels.insert("try_readlink", items.len());
    items.push(StubItem::Raw(0x9280_0c60)); // mov x0, #-100 (AT_FDCWD)
    items.push(StubItem::Raw(0xaa13_03e1)); // mov x1, x19
    items.push(StubItem::Raw(0x9100_83e2)); // add x2, sp, #0x20
    items.push(StubItem::Raw(0xd280_3fe3)); // mov x3, #511
    items.push(StubItem::Raw(0xd280_09c8)); // mov x8, #78 (__NR_readlinkat)
    items.push(StubItem::Raw(0xd400_0001)); // svc #0
    items.push(StubItem::Raw(0xf100_041f)); // cmp x0, #1
    items.push(StubItem::BCond("call_orig", 0xb)); // b.lt
    items.push(StubItem::Raw(0xf107_fc1f)); // cmp x0, #511
    items.push(StubItem::BCond("call_orig", 0xa)); // b.ge
    items.push(StubItem::Raw(0x9100_83e3)); // add x3, sp, #0x20
    items.push(StubItem::Raw(0x3820_687f)); // strb wzr, [x3, x0]
    items.push(StubItem::Adrp(2, fd35_va));
    items.push(StubItem::Raw(encode_add_imm12(
        2,
        2,
        (fd35_va & 0xfff) as u16,
    )?));

    labels.insert("cmp2", items.len());
    items.push(StubItem::Raw(0x3840_1444)); // ldrb w4, [x2], #1
    items.push(StubItem::Cbz("cmp2_end", 4, false, false));
    items.push(StubItem::Raw(0x3840_1465)); // ldrb w5, [x3], #1
    items.push(StubItem::Raw(0x6b05_009f)); // cmp w4, w5
    items.push(StubItem::BCond("cmp2", 0));
    items.push(StubItem::B("call_orig"));

    labels.insert("cmp2_end", items.len());
    items.push(StubItem::Raw(0x3940_0065)); // ldrb w5, [x3]
    items.push(StubItem::Cbz("use_buf", 5, false, false));
    items.push(StubItem::Raw(0x7100_bcbf)); // cmp w5, #0x2f
    items.push(StubItem::BCond("call_orig", 1)); // b.ne

    labels.insert("use_buf", items.len());
    items.push(StubItem::Raw(0x9100_83e9)); // add x9, sp, #0x20
    items.push(StubItem::B("check_exists"));

    labels.insert("use_x19", items.len());
    items.push(StubItem::Raw(0xaa13_03e9)); // mov x9, x19

    labels.insert("check_exists", items.len());
    items.push(StubItem::Raw(0xaa09_03e1)); // mov x1, x9
    items.push(StubItem::Raw(0x9280_0c60)); // mov x0, #-100
    items.push(StubItem::Raw(0xd280_0002)); // mov x2, #0
    items.push(StubItem::Raw(0xd280_0003)); // mov x3, #0
    items.push(StubItem::Raw(0xd280_0608)); // mov x8, #48 (__NR_faccessat)
    items.push(StubItem::Raw(0xd400_0001)); // svc #0
    items.push(StubItem::Cbz("accessible", 0, false, true));

    labels.insert("call_orig", items.len());
    items.push(StubItem::Raw(0xaa13_03e0)); // mov x0, x19
    items.push(StubItem::Raw(0xaa14_03e1)); // mov x1, x20

    labels.insert("call_orig_raw", items.len());
    items.push(StubItem::Raw(0xa941_53f3)); // ldp x19, x20, [sp, #0x10]
    items.push(StubItem::Raw(0xa940_7bfd)); // ldp x29, x30, [sp]
    items.push(StubItem::Raw(0x9108_83ff)); // add sp, sp, #0x220
    items.push(StubItem::Raw(orig_insn0));
    let realpath_resume = realpath_va + 4;
    items.push(StubItem::Adrp(16, realpath_resume));
    items.push(StubItem::Raw(encode_add_imm12(
        16,
        16,
        (realpath_resume & 0xfff) as u16,
    )?));
    items.push(StubItem::Raw(0xd61f_0200)); // br x16

    labels.insert("accessible", items.len());
    items.push(StubItem::Cbz("do_strdup", 20, false, true));
    items.push(StubItem::Raw(0xaa14_03e2)); // mov x2, x20

    labels.insert("strcpy_loop", items.len());
    items.push(StubItem::Raw(0x3840_1524)); // ldrb w4, [x9], #1
    items.push(StubItem::Raw(0x3800_1444)); // strb w4, [x2], #1
    items.push(StubItem::Cbz("strcpy_loop", 4, true, false));
    items.push(StubItem::Raw(0xaa14_03e0)); // mov x0, x20
    items.push(StubItem::B("ret_clean"));

    labels.insert("do_strdup", items.len());
    items.push(StubItem::Raw(0xaa09_03e0)); // mov x0, x9
    items.push(StubItem::Adrp(16, strdup_va));
    items.push(StubItem::Raw(encode_add_imm12(
        16,
        16,
        (strdup_va & 0xfff) as u16,
    )?));
    items.push(StubItem::Raw(0xd63f_0200)); // blr x16

    labels.insert("ret_clean", items.len());
    items.push(StubItem::Raw(0xa941_53f3)); // ldp x19, x20, [sp, #0x10]
    items.push(StubItem::Raw(0xa940_7bfd)); // ldp x29, x30, [sp]
    items.push(StubItem::Raw(0x9108_83ff)); // add sp, sp, #0x220
    items.push(StubItem::Raw(0xd65f_03c0)); // ret

    let mut out = Vec::with_capacity(items.len() * 4);
    for (idx, item) in items.iter().enumerate() {
        let pc = stub_va + (idx as u64) * 4;
        let w = match *item {
            StubItem::Raw(raw) => raw,
            StubItem::B(lbl) => encode_b(pc, label_va(lbl, &labels), false)?,
            StubItem::BCond(lbl, cond) => encode_b_cond(pc, label_va(lbl, &labels), cond)?,
            StubItem::Cbz(lbl, rt, nz, is64) => {
                encode_cbz(pc, label_va(lbl, &labels), rt, nz, is64)?
            }
            StubItem::Adrp(rd, target) => encode_adrp(rd, pc, target)?,
        };
        out.extend_from_slice(&w.to_le_bytes());
    }
    Ok(out)
}

pub(super) fn apply_uds_fd35(bytes: &mut [u8]) -> Result<usize, &'static str> {
    let elf = parse_elf(bytes)?;
    let site2 = locate_site2(bytes, &elf)?;
    let site1 = locate_site1(bytes, &elf, site2.adrp_off)?;
    let realpath = locate_realpath_and_strdup(
        bytes,
        &elf,
        site1.start_off + site1.span_insns * 4,
        site2.adrp_off,
    )?;
    let cave = locate_rx_cave(bytes, &elf, realpath.realpath_off)?;

    let va_delta = elf.rx_load.delta;
    let fd35_va = off_to_va(cave.fd35_off, va_delta)?;
    let fmt_va = off_to_va(cave.fmt_off, va_delta)?;
    let stub_va = off_to_va(cave.stub_off, va_delta)?;
    let site1_va = off_to_va(site1.start_off, va_delta)?;
    let site2_va = off_to_va(site2.adrp_off, va_delta)?;
    let realpath_va = off_to_va(realpath.realpath_off, va_delta)?;
    let strdup_va = off_to_va(realpath.strdup_off, va_delta)?;

    let stub_bytes = build_realpath_stub(
        stub_va,
        fd35_va,
        realpath_va,
        realpath.orig_insn0,
        strdup_va,
    )?;
    if stub_bytes.len() > cave.stub_cap {
        return Err("realpath FD35 wrapper stub exceeds RX cave capacity");
    }
    let realpath_hook = encode_b(realpath_va, stub_va, false)?.to_le_bytes();

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

    let mut edits: Vec<(usize, Vec<u8>)> = vec![
        (site1.start_off, site1_bytes),
        (site2.adrp_off, adrp2.to_vec()),
        (site2.add_off, add2.to_vec()),
        (cave.fd35_off, FD35_BYTES.to_vec()),
        (cave.fmt_off, FMT_UID_ONLY.to_vec()),
        (cave.stub_off, stub_bytes),
        (realpath.realpath_off, realpath_hook.to_vec()),
    ];
    if let Some(new_rx_end) = cave.extend_rx_to {
        let new_rx_sz = u64::try_from(new_rx_end - elf.rx_load.off)
            .map_err(|_| "extended RX PT_LOAD size overflow")?;
        let mut sz_pair = [0u8; 16];
        sz_pair[..8].copy_from_slice(&new_rx_sz.to_le_bytes());
        sz_pair[8..16].copy_from_slice(&new_rx_sz.to_le_bytes());
        edits.push((elf.rx_load.phdr_off + 32, sz_pair.to_vec()));
    }
    edits.sort_by_key(|(off, _)| *off);
    for pair in edits.windows(2) {
        if pair[0].0 + pair[0].1.len() > pair[1].0 {
            return Err("FD 35 UDS patch regions overlap");
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
        bytes[off..off + after.len()].copy_from_slice(&after);
    }
    if changed == 0 {
        return Err("FD 35 UDS patch made no modifications");
    }
    Ok(changed)
}

#[cfg(test)]
pub(crate) fn write_synthetic_realpath_fixture(
    buf: &mut [u8],
    rx_va_base: u64,
    text_off: usize,
    call_off: Option<usize>,
    use_thunk: bool,
    inverted: bool,
    bad_entry: bool,
) {
    let write_w = |b: &mut [u8], off: usize, v: u32| {
        b[off..off + 4].copy_from_slice(&v.to_le_bytes());
    };
    let realpath_off = text_off + 0x180;
    let thunk_off = text_off + 0x280;
    let memcpy_off = text_off + 0x2c0;
    let strdup_off = text_off + 0x300;

    let realpath_va = rx_va_base + realpath_off as u64;
    let thunk_va = rx_va_base + thunk_off as u64;
    let memcpy_va = rx_va_base + memcpy_off as u64;
    let strdup_va = rx_va_base + strdup_off as u64;

    if let Some(call_off) = call_off {
        let call_va = rx_va_base + call_off as u64;
        write_w(buf, call_off, 0xaa1f_03e1); // mov x1, xzr
        write_w(
            buf,
            call_off + 4,
            encode_b(
                call_va + 4,
                if use_thunk { thunk_va } else { realpath_va },
                true,
            )
            .unwrap(),
        );
        write_w(
            buf,
            call_off + 8,
            encode_cbz(call_va + 8, call_va + 0x14, 0, inverted, true).unwrap(),
        );
    }

    write_w(
        buf,
        thunk_off,
        encode_adrp(16, thunk_va, realpath_va).unwrap(),
    );
    write_w(
        buf,
        thunk_off + 4,
        encode_add_imm12(16, 16, (realpath_va & 0xfff) as u16).unwrap(),
    );
    write_w(buf, thunk_off + 8, 0xd61f_0200); // br x16
    write_w(buf, memcpy_off, 0xd65f_03c0);
    write_w(buf, strdup_off, 0xd65f_03c0);

    let entry_insn = if bad_entry {
        encode_b(realpath_va, realpath_va + 8, false).unwrap()
    } else {
        0xd284_140c // mov x12, #0x20a0
    };
    write_w(buf, realpath_off, entry_insn);
    write_w(buf, realpath_off + 4, 0xcb2c_63ff); // sub sp, sp, x12
    write_w(buf, realpath_off + 8, 0x7100_581f); // cmp w0, #0x16 (EINVAL check)
    if inverted {
        let memcpy_blk_off = realpath_off + 0x24;
        let memcpy_blk_va = rx_va_base + memcpy_blk_off as u64;
        let cbnz_off = realpath_off + 12;
        write_w(
            buf,
            cbnz_off,
            encode_cbz(rx_va_base + cbnz_off as u64, memcpy_blk_va, 0, true, true).unwrap(),
        );
        write_w(buf, realpath_off + 16, 0xaa15_03e0); // mov x0, x21
        write_w(
            buf,
            realpath_off + 20,
            encode_b(rx_va_base + (realpath_off + 20) as u64, strdup_va, true).unwrap(),
        );
        write_w(buf, realpath_off + 24, 0xd65f_03c0);
        write_w(buf, memcpy_blk_off, 0x9100_0662); // add x2, x19, #1
        write_w(buf, memcpy_blk_off + 4, 0xaa15_03e1); // mov x1, x21
        write_w(
            buf,
            memcpy_blk_off + 8,
            encode_b(rx_va_base + (memcpy_blk_off + 8) as u64, memcpy_va, true).unwrap(),
        );
        write_w(buf, memcpy_blk_off + 12, 0xd65f_03c0);
    } else {
        let dup_blk_off = realpath_off + 0x28;
        let dup_blk_va = rx_va_base + dup_blk_off as u64;
        let cbz_off = realpath_off + 12;
        write_w(
            buf,
            cbz_off,
            encode_cbz(rx_va_base + cbz_off as u64, dup_blk_va, 0, false, true).unwrap(),
        );
        write_w(buf, realpath_off + 16, 0x9100_0662); // add x2, x19, #1
        write_w(buf, realpath_off + 20, 0xaa15_03e1); // mov x1, x21
        write_w(
            buf,
            realpath_off + 24,
            encode_b(rx_va_base + (realpath_off + 24) as u64, memcpy_va, true).unwrap(),
        );
        write_w(buf, realpath_off + 28, 0xd65f_03c0);
        write_w(buf, dup_blk_off, 0xaa15_03e0); // mov x0, x21
        write_w(
            buf,
            dup_blk_off + 4,
            encode_b(rx_va_base + (dup_blk_off + 4) as u64, strdup_va, true).unwrap(),
        );
        write_w(buf, dup_blk_off + 8, 0xd65f_03c0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy)]
    enum Site1Variant {
        Canonical,
        RelocatedCaveAndReordered {
            mov_reg: u8,
            sp_imm: u16,
            interleaved: Option<u32>,
        },
        DuplicateSite1,
        BadRealpathEntry,
        MissingRealpathCall,
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
        let mut write_shdr =
            |idx: usize, name_off: u32, sh_type: u32, sh_flags: u64, off: usize, sz: usize| {
                let s = sh_off + idx * 64;
                write_u32(&mut buf, s, name_off);
                write_u32(&mut buf, s + 4, sh_type);
                write_u64(&mut buf, s + 8, sh_flags);
                write_u64(&mut buf, s + 24, off as u64);
                write_u64(&mut buf, s + 32, sz as u64);
            };
        write_shdr(1, 1, 1, 6, text_off, text_sz);
        write_shdr(2, 7, 1, 6, fini_off, fini_sz);
        write_shdr(3, 13, 1, 2, ro_off, ro_sz);
        write_shdr(4, 21, 3, 0, shstr_off, shstr.len());

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
            Site1Variant::Canonical
            | Site1Variant::BadRealpathEntry
            | Site1Variant::MissingRealpathCall => {
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

        let is_inverted = matches!(variant, Site1Variant::RelocatedCaveAndReordered { .. });
        write_synthetic_realpath_fixture(
            &mut buf,
            rx_va_base,
            text_off,
            (!matches!(variant, Site1Variant::MissingRealpathCall)).then_some(text_off + 0x110),
            is_inverted,
            is_inverted,
            matches!(variant, Site1Variant::BadRealpathEntry),
        );
        (buf, site1_off, site2_off, fini_off + fini_sz)
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn emitted_realpath_hook_preserves_fd35_aliases_and_libc_failures() {
        use std::ffi::{CStr, CString};
        use std::os::fd::AsRawFd;
        use std::os::unix::{ffi::OsStrExt, fs::symlink, process::CommandExt};

        const CHILD_ROOT: &str = "CODEX_TEST_REALPATH_HOOK_ROOT";
        if let Some(root) = std::env::var_os(CHILD_ROOT) {
            let root = std::path::PathBuf::from(root);
            let size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
            let mapping = unsafe {
                libc::mmap(
                    std::ptr::null_mut(),
                    size,
                    libc::PROT_READ | libc::PROT_WRITE,
                    libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                    -1,
                    0,
                )
            };
            assert_ne!(mapping, libc::MAP_FAILED);
            let base = mapping as u64;
            let mut image = vec![0u8; size];
            image[..FD35_BYTES.len()].copy_from_slice(FD35_BYTES);
            // Local absolute trampolines keep ADRP targets within the same page,
            // while fallback and allocation use the platform's real libc.
            let jump = |target: u64| -> Vec<u8> {
                let mut words = vec![0xd280_0010 | (((target & 0xffff) as u32) << 5)];
                for shift in 1..=3 {
                    words.push(
                        0xf280_0010
                            | (shift << 21)
                            | ((((target >> (shift * 16)) & 0xffff) as u32) << 5),
                    );
                }
                words.push(0xd61f_0200); // br x16
                words.into_iter().flat_map(u32::to_le_bytes).collect()
            };
            image[0x200..0x204].copy_from_slice(&NOP_INSN.to_le_bytes());
            let original = jump(libc::realpath as *const () as u64);
            image[0x204..0x204 + original.len()].copy_from_slice(&original);
            let strdup = jump(libc::strdup as *const () as u64);
            image[0x240..0x240 + strdup.len()].copy_from_slice(&strdup);
            let stub = build_realpath_stub(base + 0x40, base, base + 0x200, NOP_INSN, base + 0x240)
                .unwrap();
            assert!(stub.len() <= REALPATH_STUB_MAX_BYTES);
            image[0x40..0x40 + stub.len()].copy_from_slice(&stub);
            unsafe {
                std::ptr::copy_nonoverlapping(image.as_ptr(), mapping.cast(), size);
                // Synchronize generated instructions without a toolchain-specific
                // __clear_cache library dependency. CTR_EL0 reports line sizes.
                let ctr: u64;
                std::arch::asm!("mrs {ctr}, ctr_el0", ctr = out(reg) ctr, options(nostack));
                let start = mapping as usize;
                let end = start + size;
                let data_line = 4usize << ((ctr >> 16) & 15);
                let instruction_line = 4usize << (ctr & 15);
                for address in (start & !(data_line - 1)..end).step_by(data_line) {
                    std::arch::asm!("dc cvau, {address}", address = in(reg) address, options(nostack));
                }
                std::arch::asm!("dsb ish", options(nostack));
                for address in (start & !(instruction_line - 1)..end).step_by(instruction_line) {
                    std::arch::asm!("ic ivau, {address}", address = in(reg) address, options(nostack));
                }
                std::arch::asm!("dsb ish", "isb", options(nostack));
                assert_eq!(
                    libc::mprotect(mapping, size, libc::PROT_READ | libc::PROT_EXEC),
                    0
                );
            }
            type Realpath =
                unsafe extern "C" fn(*const libc::c_char, *mut libc::c_char) -> *mut libc::c_char;
            let hook: Realpath = unsafe { std::mem::transmute((base + 0x40) as usize) };
            let path_bytes = |path: &std::path::Path| path.as_os_str().as_bytes().to_vec();
            let plain = path_bytes(&root.join("plain"));
            let cases = [
                (
                    b"/proc/self/fd/35".to_vec(),
                    Some(b"/proc/self/fd/35".to_vec()),
                ),
                (
                    b"/proc/self/fd/35/plain".to_vec(),
                    Some(b"/proc/self/fd/35/plain".to_vec()),
                ),
                (
                    path_bytes(&root.join("alias")),
                    Some(b"/proc/self/fd/35/plain".to_vec()),
                ),
                (path_bytes(&root.join("ordinary")), Some(plain.clone())),
                (plain, Some(path_bytes(&root.join("plain")))),
                (b"/proc/self/fd/35/missing".to_vec(), None),
                (b"/proc/self/fd/35suffix".to_vec(), None),
                (path_bytes(&root.join("dangling")), None),
            ];
            for (path, expected) in cases {
                let path = CString::new(path).unwrap();
                for allocate in [false, true] {
                    let mut buffer = vec![0 as libc::c_char; 4096];
                    let destination = if allocate {
                        std::ptr::null_mut()
                    } else {
                        buffer.as_mut_ptr()
                    };
                    let result = unsafe { hook(path.as_ptr(), destination) };
                    match &expected {
                        Some(expected) => {
                            assert!(!result.is_null(), "{path:?}");
                            assert_eq!(unsafe { CStr::from_ptr(result) }.to_bytes(), expected);
                            if allocate {
                                unsafe { libc::free(result.cast()) };
                            } else {
                                assert_eq!(result, destination);
                            }
                        }
                        None => assert!(result.is_null(), "{path:?}"),
                    }
                }
            }
            assert!(unsafe { hook(std::ptr::null(), std::ptr::null_mut()) }.is_null());
            assert_eq!(unsafe { libc::close(35) }, 0);
            let absent = CString::new("/proc/self/fd/35").unwrap();
            assert!(unsafe { hook(absent.as_ptr(), std::ptr::null_mut()) }.is_null());
            assert_eq!(unsafe { libc::munmap(mapping, size) }, 0);
            return;
        }

        // FD 35 changes only in a dedicated exec child, never in the test runner.
        let root = crate::tests::temp_dir("native-realpath");
        std::fs::write(root.join("plain"), b"ok").unwrap();
        symlink("/proc/self/fd/35/plain", root.join("alias")).unwrap();
        symlink(root.join("plain"), root.join("ordinary")).unwrap();
        symlink("/proc/self/fd/35/missing", root.join("dangling")).unwrap();
        let fd = std::fs::File::open(&root).unwrap();
        let source = fd.as_raw_fd();
        let mut command = std::process::Command::new(std::env::current_exe().unwrap());
        command.args(["--exact", "runtime_patch::tests::emitted_realpath_hook_preserves_fd35_aliases_and_libc_failures", "--nocapture"])
            .env(CHILD_ROOT, &root);
        unsafe {
            command.pre_exec(move || {
                if libc::dup2(source, 35) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::fcntl(35, libc::F_SETFD, 0) < 0 {
                    return Err(std::io::Error::last_os_error());
                }
                Ok(())
            });
        }
        let status = command.status().unwrap();
        std::fs::remove_dir_all(root).unwrap();
        assert!(status.success(), "native realpath hook child: {status}");
    }

    #[test]
    fn uds_fd35_and_fd_remaps_handle_relocation_reordering_va_shifts_and_rejects_invalid() {
        let (mut canonical, site1_off, site2_off, fini_end) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        let before_canonical = canonical.clone();
        let fd_changed = crate::apply_fd_path_remaps(&mut canonical).unwrap();
        let uds_changed = apply_uds_fd35(&mut canonical).unwrap();
        let canonical_diffs = before_canonical
            .iter()
            .zip(canonical.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert!(fd_changed > 0);
        assert!(uds_changed > 0);
        assert_eq!(canonical_diffs, fd_changed + uds_changed);
        assert!(canonical
            .windows(b"/proc/self/fd/33".len())
            .any(|w| w == b"/proc/self/fd/33"));
        assert!(!canonical
            .windows(b"/etc/resolv.conf".len())
            .any(|w| w == b"/etc/resolv.conf"));
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
        let can_rp_off = 0x1180usize;
        let can_rp_va = 0x400_000u64 + can_rp_off as u64;
        let can_stub_off = cave0_fd35 + FD35_CAVE_PREFIX;
        let can_stub_va = 0x400_000u64 + can_stub_off as u64;
        assert_eq!(
            le_u32(&canonical, can_rp_off),
            Some(encode_b(can_rp_va, can_stub_va, false).unwrap())
        );
        // Re-applying fails closed after site2 and site1 are already patched.
        assert!(apply_uds_fd35(&mut canonical).is_err());
        assert_ne!(site2_off, 0);

        // Relocated cave + reordered site1 with safe interleaved `sub x8, x29, #0x18` + shifted VA base + thunk/inverted realpath.
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
        let shifted_fd_changed = crate::apply_fd_path_remaps(&mut shifted).unwrap();
        let shifted_uds_changed = apply_uds_fd35(&mut shifted).unwrap();
        let actual_shifted_diffs = before_shifted
            .iter()
            .zip(shifted.iter())
            .filter(|(a, b)| a != b)
            .count();
        assert_eq!(
            shifted_fd_changed + shifted_uds_changed,
            actual_shifted_diffs
        );
        assert!(shifted_uds_changed > 0);
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
        let sh_rp_va = 0x410_000u64 + 0x1180;
        let sh_stub_va = 0x410_000u64 + (moved_cave + FD35_CAVE_PREFIX) as u64;
        assert_eq!(
            le_u32(&shifted, 0x1180),
            Some(encode_b(sh_rp_va, sh_stub_va, false).unwrap())
        );

        // Negative cases fail closed before mutation:
        let (mut no_cave, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, true, Site1Variant::Canonical);
        assert!(apply_uds_fd35(&mut no_cave).is_err());

        // Cave large enough for strings (64B) but too small for stub must also fail.
        let (mut tiny_cave, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        tiny_cave[cave0_fd35 + 64..0x2000].fill(0x55);
        assert!(apply_uds_fd35(&mut tiny_cave).is_err());

        let (mut with_interp, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        write_u32(&mut with_interp, 64, 3); // PT_INTERP in phdr0
        assert!(apply_uds_fd35(&mut with_interp).is_err());

        let (mut bad_arch, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        write_u16(&mut bad_arch, 18, 62); // x86_64
        assert!(apply_uds_fd35(&mut bad_arch).is_err());

        let (mut dup_site1, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::DuplicateSite1);
        assert!(apply_uds_fd35(&mut dup_site1).is_err());

        let (mut missing_rp, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::MissingRealpathCall);
        assert!(apply_uds_fd35(&mut missing_rp).is_err());

        let (mut bad_rp_entry, _, _, _) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::BadRealpathEntry);
        assert!(apply_uds_fd35(&mut bad_rp_entry).is_err());

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
        assert!(apply_uds_fd35(&mut unsafe_interleaved).is_err());

        // Multi-section ELF with tiny .fini..rodata gap falls back to .init..text without PHDR edits,
        // or extends RX PT_LOAD into safe trailing zeros when no internal gap suffices.
        #[allow(clippy::too_many_arguments)]
        fn build_extended_rx_elf(
            init_gap: usize,
            fini_gap: usize,
            trailing_zeros: usize,
            rw_page_gap: u64,
        ) -> Vec<u8> {
            let va_base = 0x400_000u64;
            let init_off = 0x200usize;
            let init_sz = 0x10usize;
            let text_off = (init_off + init_sz + init_gap + 15) & !15;
            let text_sz = 0x400usize;
            let fini_off = text_off + text_sz;
            let fini_sz = 0x10usize;
            let ro_off = fini_off + fini_sz + fini_gap;
            let ro_sz = 0x200usize;
            let rx_filesz = ro_off + ro_sz;
            let rw_off = rx_filesz + trailing_zeros;
            let rw_va = va_base + rx_filesz as u64 + rw_page_gap;
            let rw_sz = 0x40usize;
            let shstr_off = rw_off + rw_sz;
            let shstr = b"\0.init\0.text\0.fini\0.rodata\0.tdata\0.shstrtab\0";
            let sh_off = (shstr_off + shstr.len() + 7) & !7;
            let total = sh_off + 7 * 64;
            let mut buf = vec![0u8; total];

            buf[..4].copy_from_slice(b"\x7fELF");
            buf[4] = 2;
            buf[5] = 1;
            buf[6] = 1;
            write_u16(&mut buf, 16, 2);
            write_u16(&mut buf, 18, 183);
            write_u32(&mut buf, 20, 1);
            write_u64(&mut buf, 32, 64);
            write_u64(&mut buf, 40, sh_off as u64);
            write_u16(&mut buf, 52, 64);
            write_u16(&mut buf, 54, 56);
            write_u16(&mut buf, 56, 2);
            write_u16(&mut buf, 58, 64);
            write_u16(&mut buf, 60, 7);
            write_u16(&mut buf, 62, 6);

            // PT_LOAD[0] RX
            write_u32(&mut buf, 64, 1);
            write_u32(&mut buf, 68, 5);
            write_u64(&mut buf, 72, 0);
            write_u64(&mut buf, 80, va_base);
            write_u64(&mut buf, 88, va_base);
            write_u64(&mut buf, 96, rx_filesz as u64);
            write_u64(&mut buf, 104, rx_filesz as u64);
            write_u64(&mut buf, 112, 0x1000);

            // PT_LOAD[1] RW
            let ph1 = 64 + 56;
            write_u32(&mut buf, ph1, 1);
            write_u32(&mut buf, ph1 + 4, 6);
            write_u64(&mut buf, ph1 + 8, rw_off as u64);
            write_u64(&mut buf, ph1 + 16, rw_va);
            write_u64(&mut buf, ph1 + 24, rw_va);
            write_u64(&mut buf, ph1 + 32, rw_sz as u64);
            write_u64(&mut buf, ph1 + 40, rw_sz as u64);
            write_u64(&mut buf, ph1 + 48, 0x1000);

            let fmt_str_off = ro_off + 0x10;
            buf[fmt_str_off..fmt_str_off + DAEMON_FMT_NEEDLE.len()]
                .copy_from_slice(DAEMON_FMT_NEEDLE);
            buf[rw_off..rw_off + 4].copy_from_slice(&[1, 2, 3, 4]);
            buf[shstr_off..shstr_off + shstr.len()].copy_from_slice(shstr);

            let mut write_shdr =
                |idx: usize, name_off: u32, sh_type: u32, sh_flags: u64, off: usize, sz: usize| {
                    let s = sh_off + idx * 64;
                    write_u32(&mut buf, s, name_off);
                    write_u32(&mut buf, s + 4, sh_type);
                    write_u64(&mut buf, s + 8, sh_flags);
                    write_u64(&mut buf, s + 24, off as u64);
                    write_u64(&mut buf, s + 32, sz as u64);
                };
            write_shdr(1, 1, 1, 6, init_off, init_sz);
            write_shdr(2, 7, 1, 6, text_off, text_sz);
            write_shdr(3, 13, 1, 6, fini_off, fini_sz);
            write_shdr(4, 19, 1, 2, ro_off, ro_sz);
            write_shdr(5, 27, 1, 3, rw_off, rw_sz);
            write_shdr(6, 34, 3, 0, shstr_off, shstr.len());

            let site1_off = text_off + 0x40;
            let site2_off = text_off + 0x200;
            let site1_insns = [
                0x528e_85e9u32,
                0x9100_83e0,
                0x72ae_0da9,
                0x5280_00a1,
                0x3900_93ff,
                0xb900_23e9,
            ];
            for (i, w) in site1_insns.iter().enumerate() {
                write_u32(&mut buf, site1_off + i * 4, *w);
            }
            let fmt_va = va_base + fmt_str_off as u64;
            write_u32(
                &mut buf,
                site2_off,
                encode_adrp(8, va_base + site2_off as u64, fmt_va).unwrap(),
            );
            write_u32(
                &mut buf,
                site2_off + 4,
                encode_add_imm12(8, 8, (fmt_va & 0xfff) as u16).unwrap(),
            );
            write_synthetic_realpath_fixture(
                &mut buf,
                va_base,
                text_off,
                Some(text_off + 0x100),
                false,
                false,
                false,
            );
            buf
        }

        // Case A: `.init..text` has >= 0x170+32 zeros, `.fini..rodata` only 0x40 -> uses `.init..text`, PHDR untouched.
        let mut init_gap_elf = build_extended_rx_elf(0x300, 0x40, 0x100, 0x10000);
        let orig_rx_filesz = le_u64(&init_gap_elf, 96).unwrap();
        assert!(apply_uds_fd35(&mut init_gap_elf).is_ok());
        assert_eq!(le_u64(&init_gap_elf, 96), Some(orig_rx_filesz));

        // Case B: small `.init..text` (0x60) & tiny `.fini..rodata` (0x40) -> extends RX PT_LOAD into 0x400 trailing zeros.
        let mut trail_elf = build_extended_rx_elf(0x60, 0x40, 0x400, 0x10000);
        let trail_orig_filesz = le_u64(&trail_elf, 96).unwrap();
        assert!(apply_uds_fd35(&mut trail_elf).is_ok());
        let trail_new_filesz = le_u64(&trail_elf, 96).unwrap();
        assert!(trail_new_filesz > trail_orig_filesz);
        assert_eq!(le_u64(&trail_elf, 104), Some(trail_new_filesz));

        // Case C: trailing zeros would collide with RW PT_LOAD's VA page -> fails closed without mutating bytes.
        let mut page_blocked = build_extended_rx_elf(0x60, 0x40, 0x400, 0x80);
        let page_blocked_before = page_blocked.clone();
        assert!(apply_uds_fd35(&mut page_blocked).is_err());
        assert_eq!(page_blocked, page_blocked_before);
    }
}
