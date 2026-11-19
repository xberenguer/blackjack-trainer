//! Opaque browser storage records. JS transports bytes; Rust owns identities,
//! integrity checks, bounded lengths, slot ordering and conflict detection.
#![no_std]
#![forbid(unsafe_code)]
pub const HEADER: usize = 64;
pub const FOOTER: usize = 24;
pub const MAX_PUBLIC: usize = 16 * 1024;
pub const MAX_PRIVATE: usize = 512 * 1024;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Identity {
    pub generation: u64,
    pub epoch: u64,
    pub nonce: [u8; 16],
}
impl Identity {
    pub fn next(self, new_nonce: [u8; 16], new_session: bool) -> Result<Self> {
        if new_nonce == [0; 16] {
            return Err(Error::Identity);
        }
        let rotate = new_session || self.epoch == u64::MAX || self.nonce == [0; 16];
        if rotate && new_nonce == self.nonce {
            return Err(Error::Identity);
        }
        Ok(Self {
            generation: self.generation.wrapping_add(1),
            epoch: if rotate { 1 } else { self.epoch + 1 },
            nonce: if rotate { new_nonce } else { self.nonce },
        })
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Kind {
    Public,
    Private,
    Tombstone,
}
impl Kind {
    fn tag(self) -> u32 {
        match self {
            Self::Public => 0,
            Self::Private => 1,
            Self::Tombstone => 2,
        }
    }
    fn maximum(self) -> usize {
        if self == Self::Public {
            MAX_PUBLIC
        } else {
            MAX_PRIVATE
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Bounds,
    Corrupt,
    Identity,
    Ambiguous,
    Io,
    Conflict,
}
pub type Result<T> = core::result::Result<T, Error>;
#[derive(Clone, Copy)]
pub struct Record<'a> {
    pub identity: Identity,
    pub kind: Kind,
    pub payload: &'a [u8],
    pub resume_enabled: bool,
}
pub fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    !c
}
fn u32at(b: &[u8], p: usize) -> Result<u32> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .ok_or(Error::Bounds)?
            .try_into()
            .map_err(|_| Error::Bounds)?,
    ))
}
fn u64at(b: &[u8], p: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(
        b.get(p..p + 8)
            .ok_or(Error::Bounds)?
            .try_into()
            .map_err(|_| Error::Bounds)?,
    ))
}
pub fn encode(out: &mut [u8], record: Record<'_>) -> Result<usize> {
    let n = record.payload.len();
    let total = HEADER
        .checked_add(n)
        .and_then(|v| v.checked_add(FOOTER))
        .ok_or(Error::Bounds)?;
    if n > record.kind.maximum()
        || total > out.len()
        || record.identity.nonce == [0; 16]
        || record.kind == Kind::Tombstone && n != 0
    {
        return Err(Error::Bounds);
    }
    out[..total].fill(0);
    out[..4].copy_from_slice(b"SBLJ");
    out[4..6].copy_from_slice(&1u16.to_le_bytes());
    out[6..8].copy_from_slice(&(HEADER as u16).to_le_bytes());
    out[8..16].copy_from_slice(&record.identity.generation.to_le_bytes());
    out[16..24].copy_from_slice(&record.identity.epoch.to_le_bytes());
    out[24..40].copy_from_slice(&record.identity.nonce);
    out[40..44].copy_from_slice(&(n as u32).to_le_bytes());
    out[44..48].copy_from_slice(
        &(record.kind.tag() | (u32::from(record.resume_enabled) << 8)).to_le_bytes(),
    );
    let payload_crc = crc(record.payload);
    out[48..52].copy_from_slice(&payload_crc.to_le_bytes());
    let header_crc = crc(&out[..60]);
    out[60..64].copy_from_slice(&header_crc.to_le_bytes());
    out[HEADER..HEADER + n].copy_from_slice(record.payload);
    let f = &mut out[HEADER + n..total];
    f[..8].copy_from_slice(b"COMMIT10");
    f[8..16].copy_from_slice(&record.identity.generation.to_le_bytes());
    f[16..20].copy_from_slice(&header_crc.to_le_bytes());
    let c = crc(&f[..20]);
    f[20..24].copy_from_slice(&c.to_le_bytes());
    Ok(total)
}
pub fn decode(bytes: &[u8], public: bool) -> Result<Record<'_>> {
    if bytes.len() < HEADER + FOOTER
        || bytes.get(..4) != Some(b"SBLJ")
        || bytes.get(4..8) != Some(&[1, 0, 64, 0])
        || u32at(bytes, 60)? != crc(&bytes[..60])
        || bytes[52..60].iter().any(|&b| b != 0)
    {
        return Err(Error::Corrupt);
    }
    let flags = u32at(bytes, 44)?;
    if flags & !0x103 != 0 {
        return Err(Error::Corrupt);
    }
    let kind = match flags & 3 {
        0 => Kind::Public,
        1 => Kind::Private,
        2 => Kind::Tombstone,
        _ => return Err(Error::Corrupt),
    };
    if public != (kind == Kind::Public) {
        return Err(Error::Corrupt);
    }
    let len = u32at(bytes, 40)? as usize;
    let end = HEADER.checked_add(len).ok_or(Error::Bounds)?;
    if len > kind.maximum()
        || end.checked_add(FOOTER) != Some(bytes.len())
        || kind == Kind::Tombstone && len != 0
    {
        return Err(Error::Bounds);
    }
    let f = &bytes[end..];
    let generation = u64at(bytes, 8)?;
    if &f[..8] != b"COMMIT10"
        || u64at(f, 8)? != generation
        || u32at(f, 16)? != u32at(bytes, 60)?
        || u32at(f, 20)? != crc(&f[..20])
        || u32at(bytes, 48)? != crc(&bytes[HEADER..end])
    {
        return Err(Error::Corrupt);
    }
    let nonce: [u8; 16] = bytes[24..40].try_into().map_err(|_| Error::Bounds)?;
    if nonce == [0; 16] {
        return Err(Error::Identity);
    }
    Ok(Record {
        identity: Identity {
            generation,
            epoch: u64at(bytes, 16)?,
            nonce,
        },
        kind,
        payload: &bytes[HEADER..end],
        resume_enabled: flags & 0x100 != 0,
    })
}
#[derive(Clone, Copy)]
pub enum Slot<'a> {
    Missing,
    Bytes(&'a [u8]),
    Io,
}
/// Public corruption/read failures never silently fall back and authorize old
/// private state. Private corruption may fall back, subject to exact binding.
pub fn latest<'a>(a: Slot<'a>, b: Slot<'a>, public: bool) -> Result<Option<(usize, Record<'a>)>> {
    let read = |s| match s {
        Slot::Missing => Ok(None),
        Slot::Bytes(b) => decode(b, public).map(Some),
        Slot::Io => Err(Error::Io),
    };
    let a = read(a);
    let b = read(b);
    if matches!(a, Err(Error::Io)) || matches!(b, Err(Error::Io)) {
        return Err(Error::Io);
    }
    if public && (a.is_err() || b.is_err()) {
        return Err(Error::Corrupt);
    }
    let a = a.ok().flatten();
    let b = b.ok().flatten();
    match (a, b) {
        (None, None) => Ok(None),
        (Some(a), None) => Ok(Some((0, a))),
        (None, Some(b)) => Ok(Some((1, b))),
        (Some(a), Some(b)) => {
            let difference = b.identity.generation.wrapping_sub(a.identity.generation);
            if difference == 0 {
                if a.identity != b.identity
                    || a.kind != b.kind
                    || a.payload != b.payload
                    || a.resume_enabled != b.resume_enabled
                {
                    return Err(Error::Ambiguous);
                }
                return Ok(Some((0, a)));
            }
            if difference == 1u64 << 63 {
                return Err(Error::Ambiguous);
            }
            if (difference as i64) > 0 {
                Ok(Some((1, b)))
            } else {
                Ok(Some((0, a)))
            }
        }
    }
}
pub fn same_public(record: Record<'_>, id: Identity, raw_public: &[u8]) -> bool {
    record.kind == Kind::Public && record.identity == id && record.payload == raw_public
}

#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::vec;
    fn id(g: u64) -> Identity {
        Identity {
            generation: g,
            epoch: g,
            nonce: [7; 16],
        }
    }
    fn packet(kind: Kind, g: u64, payload: &[u8]) -> std::vec::Vec<u8> {
        let mut b = vec![0; HEADER + FOOTER + payload.len()];
        encode(
            &mut b,
            Record {
                identity: id(g),
                kind,
                payload,
                resume_enabled: true,
            },
        )
        .unwrap();
        b
    }
    #[test]
    fn records_are_bounded_checked_and_opaque() {
        let original = packet(Kind::Public, 1, b"public packet");
        let r = decode(&original, true).unwrap();
        assert_eq!(r.identity, id(1));
        assert_eq!(r.payload.len(), 13);
        for at in 0..original.len() {
            let mut bad = original.clone();
            bad[at] ^= 1;
            assert!(decode(&bad, true).is_err(), "byte{at}");
        }
        for n in 0..original.len() {
            assert!(decode(&original[..n], true).is_err());
        }
        let mut large = vec![0; MAX_PUBLIC + HEADER + FOOTER + 1];
        let payload = vec![0; MAX_PUBLIC + 1];
        assert_eq!(
            encode(
                &mut large,
                Record {
                    identity: id(1),
                    kind: Kind::Public,
                    payload: &payload,
                    resume_enabled: false
                }
            ),
            Err(Error::Bounds)
        );
    }
    #[test]
    fn exact_progress_and_generation_binding_prevents_stale_or_ahead_state() {
        let p = packet(Kind::Public, 9, b"new scores");
        let r = decode(&p, true).unwrap();
        assert!(same_public(r, id(9), b"new scores"));
        assert!(!same_public(r, id(8), b"new scores"));
        assert!(!same_public(r, id(10), b"new scores"));
        assert!(!same_public(r, id(9), b"old scores"));
        let mut changed = id(9);
        changed.epoch += 1;
        assert!(!same_public(r, changed, b"new scores"));
        changed = id(9);
        changed.nonce = [8; 16];
        assert!(!same_public(r, changed, b"new scores"));
    }
    #[test]
    fn empty_corrupt_io_and_tied_writers_remain_distinct() {
        let a = packet(Kind::Public, 1, b"a");
        let b = packet(Kind::Public, 2, b"b");
        assert!(
            latest(Slot::Missing, Slot::Missing, true)
                .unwrap()
                .is_none()
        );
        assert_eq!(
            latest(Slot::Bytes(&a), Slot::Io, true).err(),
            Some(Error::Io)
        );
        assert_eq!(
            latest(Slot::Bytes(&a), Slot::Bytes(b"broken"), true).err(),
            Some(Error::Corrupt)
        );
        assert_eq!(
            latest(Slot::Bytes(&a), Slot::Bytes(&b), true)
                .unwrap()
                .unwrap()
                .1
                .identity,
            id(2)
        );
        let conflict = packet(Kind::Public, 2, b"otherwriter");
        assert_eq!(
            latest(Slot::Bytes(&b), Slot::Bytes(&conflict), true).err(),
            Some(Error::Ambiguous)
        );
        let private = packet(Kind::Private, 1, b"opaque");
        assert!(
            latest(Slot::Bytes(&private), Slot::Bytes(b"broken"), false)
                .unwrap()
                .is_some()
        );
    }
    #[test]
    fn generation_wrap_epoch_rotation_and_tombstones_are_explicit() {
        let mut old = id(u64::MAX);
        old.epoch = u64::MAX;
        let next = old.next([9; 16], false).unwrap();
        assert_eq!(next.generation, 0);
        assert_eq!(next.epoch, 1);
        assert_eq!(next.nonce, [9; 16]);
        let first = packet(Kind::Public, u64::MAX, b"old");
        let second = packet(Kind::Public, 0, b"new");
        assert_eq!(
            latest(Slot::Bytes(&first), Slot::Bytes(&second), true)
                .unwrap()
                .unwrap()
                .0,
            1
        );
        let tomb = packet(Kind::Tombstone, 3, b"");
        assert_eq!(decode(&tomb, false).unwrap().kind, Kind::Tombstone);
        assert!(old.next([0; 16], true).is_err());
    }
}

/// Recover surviving public progress for an explicitly unsaved, Off boot only.
/// `tainted` forbids private restore and automatic writes, even if the older
/// valid identity happens to match an unanswered private checkpoint.
pub fn recover_public<'a>(a: Slot<'a>, b: Slot<'a>) -> Result<(Option<(usize, Record<'a>)>, bool)> {
    if matches!(a, Slot::Io) || matches!(b, Slot::Io) {
        return Err(Error::Io);
    }
    let valid = |slot| match slot {
        Slot::Bytes(bytes) => decode(bytes, true).is_ok(),
        Slot::Missing => true,
        Slot::Io => false,
    };
    let av = valid(a);
    let bv = valid(b);
    let result = latest(
        if av { a } else { Slot::Missing },
        if bv { b } else { Slot::Missing },
        true,
    )?;
    Ok((result, !av || !bv))
}

#[cfg(test)]
mod recovery_tests {
    use super::*;
    use std::vec;
    #[test]
    fn payload_corruption_recovers_public_progress_but_is_always_tainted() {
        let mut a = vec![0; 100];
        let id = Identity {
            generation: 1,
            epoch: 1,
            nonce: [1; 16],
        };
        let n = encode(
            &mut a,
            Record {
                identity: id,
                kind: Kind::Public,
                payload: b"older",
                resume_enabled: true,
            },
        )
        .unwrap();
        a.truncate(n);
        let mut b = a.clone();
        let n = encode(
            &mut b,
            Record {
                identity: Identity {
                    generation: 2,
                    epoch: 2,
                    ..id
                },
                kind: Kind::Public,
                payload: b"grade",
                resume_enabled: true,
            },
        )
        .unwrap();
        b.truncate(n);
        b[HEADER] ^= 1;
        assert_eq!(
            latest(Slot::Bytes(&a), Slot::Bytes(&b), true).err(),
            Some(Error::Corrupt)
        );
        let (record, taint) = recover_public(Slot::Bytes(&a), Slot::Bytes(&b)).unwrap();
        assert!(taint);
        assert_eq!(record.unwrap().1.payload, b"older");
        assert_eq!(
            recover_public(Slot::Bytes(&a), Slot::Io).err(),
            Some(Error::Io)
        );
    }
}
