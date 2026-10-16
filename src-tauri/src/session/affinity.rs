//! CPU topology (L3 cache groups ≈ CCDs/CCXs) and CS2 affinity.
//!
//! Only processor group 0 is considered: `SetProcessAffinityMask` cannot span
//! groups and every consumer CPU has ≤ 64 logical processors.

use serde::Serialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CcdGroup {
    pub id: u32,
    pub l3_mb: f64,
    pub mask: String,
    pub logical_count: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CpuTopology {
    pub logical_count: u32,
    pub groups: Vec<CcdGroup>,
    pub recommended_mask: String,
    pub recommended_label: String,
}

/// One L3 cache as reported by Windows (group 0 only).
#[cfg_attr(not(windows), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct L3 {
    pub mask: u64,
    pub size_bytes: u64,
}

pub(crate) fn fmt_mask(mask: u64) -> String {
    format!("0x{mask:X}")
}

/// Parse "0xFF" / "ff" / "255"-style hex masks. Decimal is not accepted to avoid
/// ambiguity: the UI always sends hex.
pub(crate) fn parse_mask(s: &str) -> Option<u64> {
    let t = s.trim();
    let hex = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")).unwrap_or(t);
    if hex.is_empty() || hex.len() > 16 {
        return None;
    }
    u64::from_str_radix(hex, 16).ok().filter(|m| *m != 0)
}

/// Build the topology from raw L3 caches and the system affinity mask.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn build(mut caches: Vec<L3>, system_mask: u64) -> CpuTopology {
    caches.retain(|c| c.mask & system_mask != 0);
    caches.sort_by_key(|c| c.mask.trailing_zeros());
    caches.dedup_by_key(|c| c.mask);
    let logical_count = system_mask.count_ones();
    let groups: Vec<CcdGroup> = caches
        .iter()
        .enumerate()
        .map(|(i, c)| CcdGroup {
            id: i as u32,
            l3_mb: (c.size_bytes as f64 / (1024.0 * 1024.0) * 10.0).round() / 10.0,
            mask: fmt_mask(c.mask & system_mask),
            logical_count: (c.mask & system_mask).count_ones(),
        })
        .collect();
    let (mask, label) = recommend(&caches, system_mask);
    CpuTopology { logical_count, groups, recommended_mask: fmt_mask(mask), recommended_label: label.into() }
}

/// Recommended affinity for CS2:
/// - Several L3 groups and one is clearly bigger (≥ 1.5× every other): that is the
///   3D V-Cache CCD on X3D dual-CCD parts — CS2 is cache-sensitive, keep it there.
/// - Several equal L3 groups: the first one. Keeping all game threads on one CCD
///   avoids cross-CCD (Infinity Fabric) latency.
/// - A single L3 (most Intel and single-CCD Ryzen): everything except logical 0/1,
///   where Windows routes most interrupts/DPCs. With ≤ 4 logical processors the
///   whole mask is kept — removing a core would cost more than it saves.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn recommend(caches: &[L3], system_mask: u64) -> (u64, &'static str) {
    let groups: Vec<L3> = caches.iter().copied().filter(|c| c.mask & system_mask != 0).collect();
    if groups.len() >= 2 {
        let mut by_size = groups.clone();
        by_size.sort_by_key(|c| std::cmp::Reverse(c.size_bytes));
        let (big, second) = (by_size[0], by_size[1]);
        if big.size_bytes as f64 >= second.size_bytes as f64 * 1.5 {
            return (big.mask & system_mask, "CCD com 3D V-Cache");
        }
        let first = groups.iter().min_by_key(|c| c.mask.trailing_zeros()).copied().unwrap_or(groups[0]);
        return (first.mask & system_mask, "Primeiro CCD — evita latência entre CCDs");
    }
    if system_mask.count_ones() > 4 {
        let without_core0 = system_mask & !0b11;
        if without_core0 != 0 {
            return (without_core0, "Todos menos o núcleo 0");
        }
    }
    (system_mask, "Todos os núcleos")
}

/// Validate a custom mask against the system mask.
#[cfg_attr(not(windows), allow(dead_code))]
pub(crate) fn validate_custom(mask: u64, system_mask: u64) -> Result<u64> {
    if mask == 0 || mask & !system_mask != 0 {
        return Err(Error::InvalidInput(format!(
            "máscara {} inválida para este CPU ({} disponível)",
            fmt_mask(mask),
            fmt_mask(system_mask)
        )));
    }
    Ok(mask)
}

#[cfg(windows)]
fn read_l3() -> Result<Vec<L3>> {
    use windows::Win32::System::SystemInformation::{
        CacheUnified, GetLogicalProcessorInformationEx, RelationCache, SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX,
    };

    let mut len = 0u32;
    // First call only reports the needed size (fails with ERROR_INSUFFICIENT_BUFFER).
    let _ = unsafe { GetLogicalProcessorInformationEx(RelationCache, None, &mut len) };
    if len == 0 {
        return Err(Error::msg("não foi possível ler a topologia do CPU"));
    }
    // u64 backing store keeps the records 8-byte aligned.
    let mut buf = vec![0u64; (len as usize).div_ceil(8)];
    unsafe {
        GetLogicalProcessorInformationEx(
            RelationCache,
            Some(buf.as_mut_ptr() as *mut SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX),
            &mut len,
        )
    }
    .map_err(|e| Error::msg(format!("topologia do CPU: {e}")))?;

    let base = buf.as_ptr() as *const u8;
    let mut off = 0usize;
    let mut out = Vec::new();
    while off + std::mem::size_of::<u32>() * 2 <= len as usize {
        // SAFETY: Windows wrote `len` bytes of variable-size records; each record's
        // `Size` field gives the offset of the next one.
        let rec = unsafe { &*(base.add(off) as *const SYSTEM_LOGICAL_PROCESSOR_INFORMATION_EX) };
        if rec.Size == 0 {
            break;
        }
        if rec.Relationship == RelationCache {
            let cache = unsafe { rec.Anonymous.Cache };
            let ga = unsafe { cache.Anonymous.GroupMask };
            if cache.Level == 3 && cache.Type == CacheUnified && ga.Group == 0 {
                out.push(L3 { mask: ga.Mask as u64, size_bytes: cache.CacheSize as u64 });
            }
        }
        off += rec.Size as usize;
    }
    Ok(out)
}

#[cfg(windows)]
fn system_mask() -> Result<u64> {
    use windows::Win32::System::Threading::{GetCurrentProcess, GetProcessAffinityMask};
    let (mut proc_mask, mut sys_mask) = (0usize, 0usize);
    unsafe { GetProcessAffinityMask(GetCurrentProcess(), &mut proc_mask, &mut sys_mask) }
        .map_err(|e| Error::msg(format!("máscara do sistema: {e}")))?;
    Ok(sys_mask as u64)
}

#[cfg(windows)]
pub fn topology() -> Result<CpuTopology> {
    Ok(build(read_l3()?, system_mask()?))
}

#[cfg(not(windows))]
pub fn topology() -> Result<CpuTopology> {
    Err(Error::Unsupported)
}

/// Resolve the mask for the configured mode. Returns (mask, PT-PT label).
pub(crate) fn resolve(mode: super::AffinityMode, custom: Option<&str>) -> Result<Option<(u64, String)>> {
    use super::AffinityMode;
    match mode {
        AffinityMode::Off => Ok(None),
        AffinityMode::Auto => {
            let t = topology()?;
            let mask = parse_mask(&t.recommended_mask).ok_or_else(|| Error::msg("máscara recomendada inválida"))?;
            Ok(Some((mask, t.recommended_label)))
        }
        AffinityMode::Custom => {
            let mask = custom.and_then(parse_mask).ok_or_else(|| Error::InvalidInput("máscara personalizada em falta".into()))?;
            #[cfg(windows)]
            let mask = validate_custom(mask, system_mask()?)?;
            Ok(Some((mask, format!("personalizada {}", fmt_mask(mask)))))
        }
    }
}

/// Pin process `pid` to `mask`.
#[cfg(windows)]
pub(crate) fn apply(pid: u32, mask: u64) -> Result<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Threading::{
        OpenProcess, SetProcessAffinityMask, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SET_INFORMATION,
    };
    let h = unsafe { OpenProcess(PROCESS_SET_INFORMATION | PROCESS_QUERY_LIMITED_INFORMATION, false, pid) }
        .map_err(|e| Error::msg(format!("não foi possível abrir o processo do CS2: {e}")))?;
    let r = unsafe { SetProcessAffinityMask(h, mask as usize) };
    let _ = unsafe { CloseHandle(h) };
    r.map_err(|e| Error::msg(format!("afinidade: {e}")))
}

#[cfg(not(windows))]
pub(crate) fn apply(_pid: u32, _mask: u64) -> Result<()> {
    Err(Error::Unsupported)
}

#[cfg(test)]
mod tests {
    use super::*;

    const MB: u64 = 1024 * 1024;

    #[test]
    fn parses_masks() {
        assert_eq!(parse_mask("0xFF"), Some(0xFF));
        assert_eq!(parse_mask(" ff00 "), Some(0xFF00));
        assert_eq!(parse_mask("0x0"), None);
        assert_eq!(parse_mask("zz"), None);
        assert_eq!(parse_mask(""), None);
        assert_eq!(parse_mask("0x11112222333344445"), None);
    }

    #[test]
    fn x3d_dual_ccd_prefers_vcache() {
        // 7950X3D: CCD0 96 MB, CCD1 32 MB, 32 logical.
        let caches = vec![L3 { mask: 0xFFFF0000, size_bytes: 32 * MB }, L3 { mask: 0x0000FFFF, size_bytes: 96 * MB }];
        let (m, label) = recommend(&caches, 0xFFFF_FFFF);
        assert_eq!(m, 0xFFFF);
        assert_eq!(label, "CCD com 3D V-Cache");
        let t = build(caches, 0xFFFF_FFFF);
        assert_eq!(t.groups.len(), 2);
        assert_eq!(t.groups[0].mask, "0xFFFF");
        assert_eq!(t.groups[0].l3_mb, 96.0);
        assert_eq!(t.recommended_mask, "0xFFFF");
    }

    #[test]
    fn equal_dual_ccd_takes_first() {
        let caches = vec![L3 { mask: 0xFF00, size_bytes: 32 * MB }, L3 { mask: 0x00FF, size_bytes: 32 * MB }];
        let (m, label) = recommend(&caches, 0xFFFF);
        assert_eq!(m, 0x00FF);
        assert!(label.starts_with("Primeiro CCD"));
    }

    #[test]
    fn single_l3_skips_core0() {
        let caches = vec![L3 { mask: 0xFFF, size_bytes: 32 * MB }];
        assert_eq!(recommend(&caches, 0xFFF), (0xFFC, "Todos menos o núcleo 0"));
        // Quad-thread CPUs keep everything.
        assert_eq!(recommend(&[L3 { mask: 0xF, size_bytes: 6 * MB }], 0xF), (0xF, "Todos os núcleos"));
    }

    #[test]
    fn custom_mask_must_be_subset() {
        assert!(validate_custom(0x3, 0xF).is_ok());
        assert!(validate_custom(0x10, 0xF).is_err());
        assert!(validate_custom(0, 0xF).is_err());
    }
}
