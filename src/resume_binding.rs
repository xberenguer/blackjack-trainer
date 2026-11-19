//! Cross-journal transaction binding. Full private snapshots can never roll back newer public progress.
use crate::{
    app::App,
    persistence,
    resume_private::{self as snapshot, Reader, Restored, Wire},
};
pub const HEADER: usize = 64;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Invalid,
    Corrupt,
    Stale,
    Capacity,
    Snapshot(snapshot::Error),
}
impl From<snapshot::Error> for Error {
    fn from(e: snapshot::Error) -> Self {
        Self::Snapshot(e)
    }
}
#[derive(Clone, Copy)]
pub struct PublicCommit<'a> {
    pub generation: u64,
    pub checkpoint_epoch: u64,
    pub transaction_session: [u8; 16],
    pub packet: &'a [u8],
}
fn u32at(b: &[u8], p: usize) -> Result<u32, Error> {
    Ok(u32::from_le_bytes(
        b.get(p..p + 4)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
    ))
}
fn u64at(b: &[u8], p: usize) -> Result<u64, Error> {
    Ok(u64::from_le_bytes(
        b.get(p..p + 8)
            .ok_or(Error::Invalid)?
            .try_into()
            .map_err(|_| Error::Invalid)?,
    ))
}
fn private_public(packet: &[u8]) -> Result<&[u8], Error> {
    if packet.len() < snapshot::HEADER
        || packet.get(..4) != Some(b"SBRS")
        || u32at(packet, 12)? as usize != packet.len() - snapshot::HEADER
        || u32at(packet, 44)? != persistence::crc32(&packet[..44])
        || u32at(packet, 40)? != persistence::crc32(&packet[snapshot::HEADER..])
    {
        return Err(Error::Corrupt);
    }
    let mut r = Reader::from_bytes(&packet[snapshot::HEADER..]);
    let mut result = None;
    while !r.finished() {
        let tag = u16::get(&mut r)?;
        let flags = u16::get(&mut r)?;
        let len = u32::get(&mut r)? as usize;
        let bytes = r.bytes(len)?;
        if tag == 2 {
            if result.is_some() || flags != 1 {
                return Err(Error::Invalid);
            }
            result = Some(bytes)
        }
    }
    result.ok_or(Error::Invalid)
}
pub fn bind_private(
    out: &mut [u8],
    public: PublicCommit<'_>,
    private: &[u8],
) -> Result<usize, Error> {
    if persistence::Save::decode(public.packet).is_none()
        || private_public(private)? != public.packet
        || u64at(private, 16)? != public.checkpoint_epoch
    {
        return Err(Error::Invalid);
    }
    let total = HEADER
        .checked_add(public.packet.len())
        .and_then(|n| n.checked_add(private.len()))
        .ok_or(Error::Capacity)?;
    if total > out.len() || total > snapshot::MAX_BYTES {
        return Err(Error::Capacity);
    }
    out[..total].fill(0);
    out[..4].copy_from_slice(b"SBTX");
    out[4..6].copy_from_slice(&1u16.to_le_bytes());
    out[6..8].copy_from_slice(&(HEADER as u16).to_le_bytes());
    out[8..16].copy_from_slice(&public.generation.to_le_bytes());
    out[16..24].copy_from_slice(&public.checkpoint_epoch.to_le_bytes());
    out[24..40].copy_from_slice(&public.transaction_session);
    out[40..44].copy_from_slice(&(public.packet.len() as u32).to_le_bytes());
    out[44..48].copy_from_slice(&(private.len() as u32).to_le_bytes());
    out[48..52].copy_from_slice(&persistence::crc32(public.packet).to_le_bytes());
    out[52..56].copy_from_slice(&persistence::crc32(private).to_le_bytes());
    out[56..60].copy_from_slice(&1u32.to_le_bytes());
    let c = persistence::crc32(&out[..60]);
    out[60..64].copy_from_slice(&c.to_le_bytes());
    out[HEADER..HEADER + public.packet.len()].copy_from_slice(public.packet);
    out[HEADER + public.packet.len()..total].copy_from_slice(private);
    Ok(total)
}
/// `latest_public` must come from a verified, committed public journal read, before normal boot-Off overrides.
/// On mismatch the caller keeps latest public progress and starts fresh; it never adds/replaces totals from private data.
pub fn restore_bound(
    candidate: &mut App,
    bundle: &[u8],
    latest_public: PublicCommit<'_>,
    clock: u32,
    enabled: bool,
) -> Result<Restored, Error> {
    if bundle.len() < HEADER
        || bundle.len() > snapshot::MAX_BYTES
        || bundle.get(..4) != Some(b"SBTX")
        || bundle.get(4..8) != Some(&[1, 0, 64, 0])
        || u32at(bundle, 56)? != 1
        || u32at(bundle, 60)? != persistence::crc32(&bundle[..60])
    {
        return Err(Error::Corrupt);
    }
    let public_len = u32at(bundle, 40)? as usize;
    let private_len = u32at(bundle, 44)? as usize;
    let split = HEADER.checked_add(public_len).ok_or(Error::Invalid)?;
    let end = split.checked_add(private_len).ok_or(Error::Invalid)?;
    if end != bundle.len() {
        return Err(Error::Invalid);
    }
    let public = bundle.get(HEADER..split).ok_or(Error::Invalid)?;
    let private = bundle.get(split..end).ok_or(Error::Invalid)?;
    if u32at(bundle, 48)? != persistence::crc32(public)
        || u32at(bundle, 52)? != persistence::crc32(private)
    {
        return Err(Error::Corrupt);
    }
    if u64at(bundle, 8)? != latest_public.generation
        || u64at(bundle, 16)? != latest_public.checkpoint_epoch
        || bundle.get(24..40) != Some(&latest_public.transaction_session)
        || public != latest_public.packet
    {
        return Err(Error::Stale);
    }
    if persistence::Save::decode(public).is_none()
        || private_public(private)? != public
        || u64at(private, 16)? != latest_public.checkpoint_epoch
    {
        return Err(Error::Invalid);
    }
    Ok(snapshot::restore_candidate(
        candidate, private, clock, enabled,
    )?)
}

/// Bounded hardware binding over a completed immutable capture. Each step copies/hashes <=512 bytes.
/// The captured buffer and committed public metadata stay exclusively borrowed until Done.
pub struct BindTask<'a> {
    out: &'a mut [u8],
    public: PublicCommit<'a>,
    private: &'a [u8],
    at: usize,
    total: usize,
    crc: u32,
    pubcrc: u32,
    payload_crc: u32,
    done: bool,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BindProgress {
    More,
    Done { bytes: usize },
}
fn update(mut crc: u32, bytes: &[u8]) -> u32 {
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = (crc >> 1) ^ if crc & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    crc
}
impl<'a> BindTask<'a> {
    pub fn new(
        out: &'a mut [u8],
        public: PublicCommit<'a>,
        private: &'a [u8],
    ) -> Result<Self, Error> {
        if private.len() < snapshot::HEADER
            || private.get(..4) != Some(b"SBRS")
            || u32at(private, 12)? as usize != private.len() - snapshot::HEADER
            || u32at(private, 44)? != persistence::crc32(&private[..44])
            || u64at(private, 16)? != public.checkpoint_epoch
            || persistence::Save::decode(public.packet).is_none()
        {
            return Err(Error::Invalid);
        }
        let mut r = Reader::from_bytes(&private[snapshot::HEADER..]);
        let mut found = false;
        while !r.finished() {
            let tag = u16::get(&mut r)?;
            let flags = u16::get(&mut r)?;
            let n = u32::get(&mut r)? as usize;
            let b = r.bytes(n)?;
            if tag == 2 {
                if found || flags != 1 || b != public.packet {
                    return Err(Error::Invalid);
                }
                found = true
            }
        }
        if !found {
            return Err(Error::Invalid);
        }
        let total = HEADER
            .checked_add(public.packet.len())
            .and_then(|n| n.checked_add(private.len()))
            .ok_or(Error::Capacity)?;
        if total > out.len() || total > snapshot::MAX_BYTES {
            return Err(Error::Capacity);
        }
        Ok(Self {
            out,
            public,
            private,
            at: 0,
            total,
            crc: !0,
            pubcrc: !0,
            payload_crc: !0,
            done: false,
        })
    }
    pub fn step(&mut self, budget: usize) -> Result<BindProgress, Error> {
        if self.done {
            return Ok(BindProgress::Done { bytes: self.total });
        }
        let budget = budget.clamp(1, 512);
        let public_len = self.public.packet.len();
        if self.at < public_len {
            let end = (self.at + budget).min(public_len);
            self.out[HEADER + self.at..HEADER + end]
                .copy_from_slice(&self.public.packet[self.at..end]);
            self.pubcrc = update(self.pubcrc, &self.public.packet[self.at..end]);
            self.at = end;
            return Ok(BindProgress::More);
        }
        let at = self.at - public_len;
        if at < self.private.len() {
            let end = (at + budget).min(self.private.len());
            let b = &self.private[at..end];
            self.out[HEADER + public_len + at..HEADER + public_len + end].copy_from_slice(b);
            self.crc = update(self.crc, b);
            if end > snapshot::HEADER {
                self.payload_crc = update(
                    self.payload_crc,
                    &self.private[at.max(snapshot::HEADER)..end],
                );
            }
            self.at = public_len + end;
            return Ok(BindProgress::More);
        }
        if !self.payload_crc != u32at(self.private, 40)? {
            return Err(Error::Corrupt);
        }
        let h = &mut self.out[..HEADER];
        h.fill(0);
        h[..4].copy_from_slice(b"SBTX");
        h[4..6].copy_from_slice(&1u16.to_le_bytes());
        h[6..8].copy_from_slice(&(HEADER as u16).to_le_bytes());
        h[8..16].copy_from_slice(&self.public.generation.to_le_bytes());
        h[16..24].copy_from_slice(&self.public.checkpoint_epoch.to_le_bytes());
        h[24..40].copy_from_slice(&self.public.transaction_session);
        h[40..44].copy_from_slice(&(public_len as u32).to_le_bytes());
        h[44..48].copy_from_slice(&(self.private.len() as u32).to_le_bytes());
        h[48..52].copy_from_slice(&(!self.pubcrc).to_le_bytes());
        h[52..56].copy_from_slice(&(!self.crc).to_le_bytes());
        h[56..60].copy_from_slice(&1u32.to_le_bytes());
        let crc = persistence::crc32(&h[..60]);
        h[60..64].copy_from_slice(&crc.to_le_bytes());
        self.done = true;
        Ok(BindProgress::Done { bytes: self.total })
    }
}
