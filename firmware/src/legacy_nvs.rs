//! Read-only ESP-IDF NVS V1/V2 single-chunk userdata importer.
use crate::storage_format as fmt;
use stardust_blackjack::persistence::Save;
pub fn import(
    bytes: u32,
    page: &mut [u8; 4096],
    mut read: impl FnMut(usize, &mut [u8; 4096]) -> bool,
) -> Option<Save> {
    let mut ns = None;
    for offset in (0..bytes).step_by(4096) {
        let p = &mut *page;
        if !read((offset / 4096) as usize, p) {
            return None;
        }
        for slot in 0..126 {
            if let Some(e) = fmt::entry(p, slot) {
                if e[0] == 0 && e[1] == 1 && e[2] == 1 && e[3] == 255 && fmt::key(e, b"stardustbj")
                {
                    ns = Some(e[24]);
                }
            }
        }
    }
    let ns = ns?;
    let mut newest_index = None;
    let mut legacy = None;
    for offset in (0..bytes).step_by(4096) {
        let p = &mut *page;
        if !read((offset / 4096) as usize, p) {
            return None;
        }
        let seq = u32::from_le_bytes(p[4..8].try_into().ok()?);
        for slot in 0..126 {
            let Some(e) = fmt::entry(p, slot) else {
                continue;
            };
            if e[0] != ns || !fmt::key(e, b"userdata") {
                continue;
            }
            if e[1] == 0x48 && e[2] == 1 && e[3] == 255 {
                let len = u32::from_le_bytes(e[24..28].try_into().ok()?) as usize;
                if fmt::accepted_size(len)
                    && e[28] == 1
                    && newest_index
                        .as_ref()
                        .is_none_or(|(s, i, _, _)| fmt::newer(seq, *s) || (seq == *s && slot > *i))
                {
                    newest_index = Some((seq, slot, e[29], len));
                }
            }
            if e[1] == 0x41 {
                if let Some(s) = blob(p, slot, e) {
                    if legacy
                        .as_ref()
                        .is_none_or(|(s0, i, _)| fmt::newer(seq, *s0) || (seq == *s0 && slot > *i))
                    {
                        legacy = Some((seq, slot, s));
                    }
                }
            }
        }
    }
    if let Some((_, _, chunk, len)) = newest_index {
        let mut found = None;
        for offset in (0..bytes).step_by(4096) {
            let p = &mut *page;
            if !read((offset / 4096) as usize, p) {
                return None;
            }
            let seq = u32::from_le_bytes(p[4..8].try_into().ok()?);
            for slot in 0..126 {
                let Some(e) = fmt::entry(p, slot) else {
                    continue;
                };
                if e[0] == ns
                    && e[1] == 0x42
                    && e[3] == chunk
                    && fmt::key(e, b"userdata")
                    && u16::from_le_bytes([e[24], e[25]]) as usize == len
                {
                    if let Some(s) = blob(p, slot, e) {
                        if found.as_ref().is_none_or(|(old, _)| fmt::newer(seq, *old)) {
                            found = Some((seq, s));
                        }
                    }
                }
            }
        }
        return found.map(|(_, s)| s);
    }
    legacy.map(|(_, _, s)| s)
}
fn blob(page: &[u8], slot: usize, e: &[u8]) -> Option<Save> {
    let n = u16::from_le_bytes([e[24], e[25]]) as usize;
    let span = e[2] as usize;
    if !fmt::accepted_size(n) || span != 1 + n.div_ceil(32) || slot.checked_add(span)? > 126 {
        return None;
    }
    for i in slot..slot + span {
        if !fmt::written(page, i) {
            return None;
        }
    }
    let data = page.get(64 + (slot + 1) * 32..64 + (slot + 1) * 32 + n)?;
    if u32::from_le_bytes(e[28..32].try_into().ok()?) != fmt::nvs_crc32(data) {
        return None;
    }
    Save::decode(data)
}
