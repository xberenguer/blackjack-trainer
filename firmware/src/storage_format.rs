//! Pure bounded storage envelopes; tested on the host without hardware.
pub const PACKET: usize = stardust_blackjack::persistence::SIZE;
pub const JOURNAL_BYTES: usize = PACKET + 72;
const _: () = assert!(JOURNAL_BYTES <= 4096);
pub fn crc32(b: &[u8]) -> u32 {
    let mut c = !0u32;
    for &v in b {
        c ^= v as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    !c
}
pub fn nvs_crc32(b: &[u8]) -> u32 {
    let mut c = 0u32;
    for &v in b {
        c ^= v as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    !c
}
fn u32at(b: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(p..p + 4)?.try_into().ok()?))
}
pub struct Encoded {
    pub bytes: [u8; JOURNAL_BYTES],
    pub len: usize,
}
pub fn accepted_size(n: usize) -> bool {
    [196, 208, 336, 584, PACKET].contains(&n)
}
pub fn record(packet: &[u8], sequence: u32) -> Option<Encoded> {
    if !accepted_size(packet.len()) || packet.len() > PACKET {
        return None;
    }
    let len = packet.len() + 24;
    let mut b = [255; JOURNAL_BYTES];
    b[..4].copy_from_slice(b"SBRJ");
    b[4..8].copy_from_slice(&1u32.to_le_bytes());
    b[8..12].copy_from_slice(&sequence.to_le_bytes());
    b[12..16].copy_from_slice(&(packet.len() as u32).to_le_bytes());
    let end = 16 + packet.len();
    b[16..end].copy_from_slice(packet);
    let c = crc32(&b[..end]);
    b[end..end + 4].copy_from_slice(&c.to_le_bytes());
    b[end + 4..end + 8].copy_from_slice(b"DONE");
    Some(Encoded { bytes: b, len })
}
pub fn decode(b: &[u8]) -> Option<(u32, &[u8])> {
    if b.get(..4)? != b"SBRJ" || u32at(b, 4)? != 1 {
        return None;
    }
    let n = u32at(b, 12)? as usize;
    if !accepted_size(n) || n > PACKET {
        return None;
    }
    let end = 16 + n;
    if b.get(end + 4..end + 8)? != b"DONE" || u32at(b, end)? != crc32(b.get(..end)?) {
        return None;
    }
    Some((u32at(b, 8)?, b.get(16..end)?))
}
pub struct Chosen {
    pub slot: usize,
    pub sequence: u32,
    pub save: stardust_blackjack::persistence::Save,
    pub current: bool,
    pub meta: Option<Meta>,
    pub resume_safe: bool,
    pub generation_floor: u64,
    pub epoch_floor: u64,
}
pub fn choose_slots(
    first: Result<&[u8], ()>,
    second: Result<&[u8], ()>,
) -> Result<Option<Chosen>, ()> {
    let slots = [first?, second?];
    let mut chosen: Option<Chosen> = None;
    for (slot, bytes) in slots.into_iter().enumerate() {
        if let Some((meta, packet)) = decode_v2(bytes) {
            if chosen.as_ref().is_none_or(|old| {
                old.meta
                    .is_none_or(|m| newer64(meta.generation, m.generation))
            }) {
                chosen = Some(Chosen {
                    slot,
                    sequence: meta.generation as u32,
                    save: stardust_blackjack::persistence::Save::decode(packet)
                        .expect("validated public packet"),
                    current: true,
                    meta: Some(meta),
                    resume_safe: true,
                    generation_floor: meta.generation,
                    epoch_floor: meta.epoch,
                });
            }
        } else if let Some((sequence, packet)) = decode(bytes) {
            if let Some(save) = stardust_blackjack::persistence::Save::decode(packet) {
                if chosen
                    .as_ref()
                    .is_none_or(|old| old.meta.is_none() && newer(sequence, old.sequence))
                {
                    chosen = Some(Chosen {
                        slot,
                        sequence,
                        save,
                        current: packet.len() == PACKET,
                        meta: None,
                        resume_safe: true,
                        generation_floor: sequence as u64,
                        epoch_floor: 0,
                    });
                }
            }
        }
    }
    let Some(mut chosen) = chosen else {
        // A recognizable committed-but-corrupt Rust record is never permission to import stale NVS.
        if slots.iter().any(|b| looks_committed(b)) {
            return Err(());
        }
        return Ok(None);
    };
    for bytes in slots {
        if decode_v2(bytes).is_some()
            || decode(bytes)
                .is_some_and(|(_, p)| stardust_blackjack::persistence::Save::decode(p).is_some())
        {
            continue;
        }
        if looks_committed(bytes) {
            chosen.resume_safe = false;
        }
        if let Some(meta) = header_hint(bytes) {
            if newer64(meta.generation, chosen.generation_floor) {
                chosen.generation_floor = meta.generation;
                chosen.epoch_floor = meta.epoch;
                chosen.resume_safe = false;
            }
        }
    }
    Ok(Some(chosen))
}
fn header_hint(b: &[u8]) -> Option<Meta> {
    if b.get(..4)? != b"SBRJ" || u32at(b, 4)? != 2 || u32at(b, 60)? != crc32(b.get(..60)?) {
        return None;
    }
    Some(Meta {
        generation: u64::from_le_bytes(b.get(8..16)?.try_into().ok()?),
        epoch: u64::from_le_bytes(b.get(16..24)?.try_into().ok()?),
        nonce: b.get(24..40)?.try_into().ok()?,
    })
}
fn looks_committed(b: &[u8]) -> bool {
    // Try supported body locations independently of header length/version, so corruption of a
    // header field cannot turn a previously acknowledged commit into a stale-NVS fallback.
    if b.get(..4) != Some(b"SBRJ") && !matches!(u32at(b, 4), Some(1 | 2)) {
        return false;
    }
    [196, 208, 336, 584, PACKET].into_iter().any(|n| {
        [16 + n + 4, 64 + n + 4]
            .into_iter()
            .any(|at| b.get(at..at + 4) == Some(b"DONE"))
    })
}

pub fn newer(a: u32, b: u32) -> bool {
    (a.wrapping_sub(b) as i32) > 0
}
pub fn key(entry: &[u8], name: &[u8]) -> bool {
    entry.get(8..8 + name.len()) == Some(name) && entry.get(8 + name.len()) == Some(&0)
}
pub fn written(page: &[u8], slot: usize) -> bool {
    slot < 126
        && page
            .get(32 + slot / 4)
            .is_some_and(|b| ((b >> (2 * (slot % 4))) & 3) == 2)
}
pub fn entry(page: &[u8], slot: usize) -> Option<&[u8]> {
    if ![0xfffffffe, 0xfffffffc, 0xfffffff8].contains(&u32at(page, 0)?)
        || ![0xfe, 0xff].contains(page.get(8)?)
        || u32at(page, 28)? != nvs_crc32(page.get(4..28)?)
    {
        return None;
    }
    if !written(page, slot) {
        return None;
    }
    let e = page.get(64 + slot * 32..96 + slot * 32)?;
    let mut b = [0u8; 28];
    b[..4].copy_from_slice(&e[..4]);
    b[4..].copy_from_slice(&e[8..]);
    if u32at(e, 4)? != nvs_crc32(&b) {
        return None;
    }
    Some(e)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn crc_vector() {
        assert_eq!(crc32(b"123456789"), 0xcbf43926);
        assert_eq!(nvs_crc32(b"123456789"), 0xd202d277)
    }
    #[test]
    fn journal_power_cut_and_corruption() {
        for n in [196, 208, 336, 584, PACKET] {
            let p = vec![42; n];
            let encoded = record(&p, u32::MAX).unwrap();
            let b = &encoded.bytes[..encoded.len];
            assert_eq!(decode(b), Some((u32::MAX, &p[..])));
            for i in 0..b.len() {
                let mut broken = b.to_vec();
                broken[i] ^= 1;
                assert!(decode(&broken).is_none(), "size{n} byte{i}");
            }
            for cut in 0..b.len() {
                assert!(decode(&b[..cut]).is_none());
            }
        }
        assert!(newer(0, u32::MAX));
        assert!(!newer(u32::MAX, 0));
    }
    #[test]
    fn bounded_entries() {
        for n in 0..4096 {
            let b = vec![0; n];
            assert!(entry(&b, 126).is_none());
            assert!(entry(&b, usize::MAX).is_none());
        }
    }
}

/// V2 public commit identity. Packet bytes stay exact (before normal boot-Off preference application).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Meta {
    pub generation: u64,
    pub epoch: u64,
    pub nonce: [u8; 16],
}
pub const V2_BYTES: usize = PACKET + 72;
const _: () = assert!(V2_BYTES <= 4096);
pub struct EncodedV2 {
    pub bytes: [u8; V2_BYTES],
    pub len: usize,
}
pub fn record_v2(packet: &[u8], meta: Meta) -> Option<EncodedV2> {
    if packet.len() != PACKET || stardust_blackjack::persistence::Save::decode(packet).is_none() {
        return None;
    }
    let mut b = [0xff; V2_BYTES];
    b[..64].fill(0);
    b[..4].copy_from_slice(b"SBRJ");
    b[4..8].copy_from_slice(&2u32.to_le_bytes());
    b[8..16].copy_from_slice(&meta.generation.to_le_bytes());
    b[16..24].copy_from_slice(&meta.epoch.to_le_bytes());
    b[24..40].copy_from_slice(&meta.nonce);
    b[40..44].copy_from_slice(&(packet.len() as u32).to_le_bytes());
    b[56..60].copy_from_slice(&crc32(packet).to_le_bytes());
    let c = crc32(&b[..60]);
    b[60..64].copy_from_slice(&c.to_le_bytes());
    let end = 64 + packet.len();
    b[64..end].copy_from_slice(packet);
    let c = crc32(&b[..end]);
    b[end..end + 4].copy_from_slice(&c.to_le_bytes());
    b[end + 4..end + 8].copy_from_slice(b"DONE");
    Some(EncodedV2 {
        bytes: b,
        len: end + 8,
    })
}
pub fn decode_v2(b: &[u8]) -> Option<(Meta, &[u8])> {
    if b.get(..4)? != b"SBRJ"
        || u32at(b, 4)? != 2
        || u32at(b, 40)? as usize != PACKET
        || b.get(44..56)? != [0; 12]
        || u32at(b, 60)? != crc32(b.get(..60)?)
    {
        return None;
    }
    let end = 64 + PACKET;
    let p = b.get(64..end)?;
    if b.get(end + 4..end + 8)? != b"DONE"
        || u32at(b, end)? != crc32(b.get(..end)?)
        || u32at(b, 56)? != crc32(p)
        || stardust_blackjack::persistence::Save::decode(p).is_none()
    {
        return None;
    }
    Some((
        Meta {
            generation: u64::from_le_bytes(b.get(8..16)?.try_into().ok()?),
            epoch: u64::from_le_bytes(b.get(16..24)?.try_into().ok()?),
            nonce: b.get(24..40)?.try_into().ok()?,
        },
        p,
    ))
}
pub fn newer64(a: u64, b: u64) -> bool {
    (a.wrapping_sub(b) as i64) > 0
}
