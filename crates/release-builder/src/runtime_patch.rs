//! Rule-based Termux adaptation of the protected UDS root.
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
    pub uds: &'static str,
}

pub(super) fn policy(id: &str) -> Option<Policy> {
    (id == ADAPTIVE_POLICY).then_some(Policy {
        id: ADAPTIVE_POLICY,
        uds: ADAPTIVE_UDS_POLICY,
    })
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
        [0, minor, _] if *minor >= 165 => Ok(policy(ADAPTIVE_POLICY)),
        [major, _, _] if *major >= 1 => Ok(policy(ADAPTIVE_POLICY)),
        _ => Ok(None),
    }
}

pub(super) fn supports_version(id: &str, version: &str) -> bool {
    if id == super::PATCH_POLICY_ID {
        required(version).is_ok_and(|opt| opt.is_none())
    } else if id == ADAPTIVE_POLICY {
        required(version).is_ok_and(|opt| opt.is_some())
    } else {
        false
    }
}

pub(super) fn contains_daemon_fmt(bytes: &[u8]) -> bool {
    parse_elf(bytes).is_ok_and(|elf| {
        bytes[elf.rodata.off..elf.rodata.off + elf.rodata.sz]
            .windows(DAEMON_FMT_NEEDLE.len())
            .any(|w| w == DAEMON_FMT_NEEDLE)
    })
}

pub(super) fn apply(bytes: &mut [u8], policy: Policy) -> Result<usize, &'static str> {
    if policy.id != ADAPTIVE_POLICY {
        return Err("unsupported Termux runtime patch policy");
    }
    apply_adaptive_uds_fd35(bytes)
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
    fn runtime_artifact_patch_policy_tracks_adaptive_versions_without_hardcoded_hashes() {
        assert_eq!(required("0.159.2"), Ok(None));
        assert_eq!(required("0.164.0"), Ok(None));
        assert!(supports_version(crate::PATCH_POLICY_ID, "0.164.0"));
        assert!(!supports_version(ADAPTIVE_POLICY, "0.164.0"));
        assert!(required("invalid").is_err());
        assert!(required("0.169").is_err());
        for adaptive in ["0.165.0", "0.167.2", "0.169.0", "0.200.1", "1.0.0"] {
            let p = required(adaptive).unwrap().unwrap();
            assert_eq!(p.id, ADAPTIVE_POLICY);
            assert!(supports_version(ADAPTIVE_POLICY, adaptive));
            assert!(!supports_version(crate::PATCH_POLICY_ID, adaptive));
        }
        assert!(!supports_version("termux-fd-remap-v5", "0.163.0"));
        assert!(!supports_version("unknown", "0.169.0"));
    }

    #[test]
    fn adaptive_99b_handles_relocation_reordering_va_shifts_and_rejects_invalid() {
        let policy = required("0.169.0").unwrap().unwrap();
        let (mut canonical, site1_off, site2_off, fini_end) =
            build_synthetic_aarch64_elf(0x400_000, false, false, Site1Variant::Canonical);
        let fd_changed = crate::apply_fd_path_remaps(&mut canonical).unwrap();
        let uds_changed = apply(&mut canonical, policy).unwrap();
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
        assert!(apply(&mut canonical, policy).is_err());
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
