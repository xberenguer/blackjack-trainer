//! Private resume codec. Opaque storage bridges may transport bytes internally; runtime USB, general WASM, status, help and download APIs must never expose them.
use crate::{app::App, persistence};
#[path = "resume_strings.rs"]
mod strings;
pub const MAX_BYTES: usize = 262_000;
pub const HEADER: usize = 48;
pub const SCHEMA: u32 = 0x000a_0001;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Truncated,
    Invalid,
    Capacity,
    Disabled,
    Corrupt,
    Schema,
    Inconsistent,
    Yield,
}
pub type Result<T> = core::result::Result<T, Error>;
pub struct Writer<'a> {
    b: &'a mut [u8],
    pub at: usize,
    skip_before: usize,
    stop_after: Option<usize>,
}
impl<'a> Writer<'a> {
    pub fn bytes(&mut self, b: &[u8]) -> Result<()> {
        let end = self.at.checked_add(b.len()).ok_or(Error::Capacity)?;
        if end > self.b.len() {
            return Err(Error::Capacity);
        }
        let limit = self.stop_after.unwrap_or(end);
        let begin = self.at.max(self.skip_before);
        let finish = end.min(limit);
        if begin < finish {
            self.b[begin..finish].copy_from_slice(&b[begin - self.at..finish - self.at]);
        }
        if end > limit {
            self.at = limit;
            return Err(Error::Yield);
        }
        self.at = end;
        Ok(())
    }
    pub fn u8(&mut self, b: u8) -> Result<()> {
        self.bytes(&[b])
    }
}
pub struct Reader<'a> {
    b: &'a [u8],
    pub at: usize,
}
impl<'a> Reader<'a> {
    pub(crate) fn from_bytes(b: &'a [u8]) -> Self {
        Self { b, at: 0 }
    }
    pub(crate) fn finished(&self) -> bool {
        self.at == self.b.len()
    }
    pub fn bytes(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.at.checked_add(n).ok_or(Error::Truncated)?;
        let out = self.b.get(self.at..end).ok_or(Error::Truncated)?;
        self.at = end;
        Ok(out)
    }
    pub fn u8(&mut self) -> Result<u8> {
        Ok(self.bytes(1)?[0])
    }
}
pub(crate) trait Wire: Sized {
    fn initial() -> Self;
    fn put(&self, w: &mut Writer) -> Result<()>;
    fn get(r: &mut Reader) -> Result<Self>;
    /// Decode into initialized owned storage. Composite overrides avoid whole-model return frames.
    fn read_into(&mut self, r: &mut Reader) -> Result<()> {
        *self = Self::get(r)?;
        Ok(())
    }
}
macro_rules! integer{($($t:ty),*)=>{$(impl Wire for $t{fn initial()->Self{0}fn put(&self,w:&mut Writer)->Result<()>{w.bytes(&self.to_le_bytes())}fn get(r:&mut Reader)->Result<Self>{Ok(Self::from_le_bytes(r.bytes(core::mem::size_of::<Self>())?.try_into().map_err(|_|Error::Truncated)?))}})*}}
integer!(u8, u16, u32, u64, i16, i64, i128);
impl Wire for usize {
    fn initial() -> Self {
        0
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        u32::try_from(*self).map_err(|_| Error::Invalid)?.put(w)
    }
    fn get(r: &mut Reader) -> Result<Self> {
        Ok(u32::get(r)? as usize)
    }
}
impl Wire for bool {
    fn initial() -> Self {
        false
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        w.u8(u8::from(*self))
    }
    fn get(r: &mut Reader) -> Result<Self> {
        match r.u8()? {
            0 => Ok(false),
            1 => Ok(true),
            _ => Err(Error::Invalid),
        }
    }
}
impl Wire for f64 {
    fn initial() -> Self {
        0.0
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        self.to_bits().put(w)
    }
    fn get(r: &mut Reader) -> Result<Self> {
        Ok(Self::from_bits(u64::get(r)?))
    }
}
impl<T: Wire> Wire for Option<T> {
    fn initial() -> Self {
        None
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        match self {
            None => w.u8(0),
            Some(v) => {
                w.u8(1)?;
                v.put(w)
            }
        }
    }
    fn get(r: &mut Reader) -> Result<Self> {
        match r.u8()? {
            0 => Ok(None),
            1 => Ok(Some(T::get(r)?)),
            _ => Err(Error::Invalid),
        }
    }
}
impl<T: Wire + Copy, const N: usize> Wire for [T; N] {
    fn initial() -> Self {
        [T::initial(); N]
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        for v in self {
            v.put(w)?
        }
        Ok(())
    }
    fn get(r: &mut Reader) -> Result<Self> {
        let mut out = Self::initial();
        for v in &mut out {
            *v = T::get(r)?
        }
        Ok(out)
    }
    fn read_into(&mut self, r: &mut Reader) -> Result<()> {
        for value in self.iter_mut() {
            value.read_into(r)?;
        }
        Ok(())
    }
}
impl<T: Wire, U: Wire> Wire for (T, U) {
    fn initial() -> Self {
        (T::initial(), U::initial())
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        self.0.put(w)?;
        self.1.put(w)
    }
    fn get(r: &mut Reader) -> Result<Self> {
        Ok((T::get(r)?, U::get(r)?))
    }
}
impl<T: Wire, U: Wire, V: Wire> Wire for (T, U, V) {
    fn initial() -> Self {
        (T::initial(), U::initial(), V::initial())
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        self.0.put(w)?;
        self.1.put(w)?;
        self.2.put(w)
    }
    fn get(r: &mut Reader) -> Result<Self> {
        Ok((T::get(r)?, U::get(r)?, V::get(r)?))
    }
}
impl Wire for &'static str {
    fn initial() -> Self {
        ""
    }
    fn put(&self, w: &mut Writer) -> Result<()> {
        let len = u16::try_from(self.len()).map_err(|_| Error::Invalid)?;
        if !strings::STRINGS.contains(self) {
            return Err(Error::Invalid);
        }
        len.put(w)?;
        w.bytes(self.as_bytes())
    }
    fn get(r: &mut Reader) -> Result<Self> {
        let n = u16::get(r)? as usize;
        let b = r.bytes(n)?;
        strings::STRINGS
            .iter()
            .copied()
            .find(|s| s.as_bytes() == b)
            .ok_or(Error::Invalid)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Restored {
    pub sequence: u64,
    pub saved_clock: u32,
    pub neutral: NeutralGate,
}
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NeutralGate {
    ready: bool,
}
impl NeutralGate {
    pub const fn new() -> Self {
        Self { ready: false }
    }
    pub fn accept(&mut self, down: [bool; 2]) -> bool {
        if !self.ready {
            if down == [false; 2] {
                self.ready = true
            }
            false
        } else {
            true
        }
    }
}
/// Safe to present only as a boolean/count/sequence result; private payload is never diagnostic output.
pub fn capture_private(app: &App, out: &mut [u8], sequence: u64, enabled: bool) -> Result<usize> {
    if !enabled {
        return Err(Error::Disabled);
    }
    if out.len() < HEADER || out.len() > MAX_BYTES {
        return Err(Error::Capacity);
    }
    if !app.resume_valid() {
        return Err(Error::Inconsistent);
    }
    let mut w = Writer {
        b: &mut out[HEADER..],
        at: 0,
        skip_before: 0,
        stop_after: None,
    };
    // Two mandatory TLVs: explicit private app fields and independent public preference packet.
    1u16.put(&mut w)?;
    1u16.put(&mut w)?;
    0u32.put(&mut w)?;
    let at = w.at;
    app.resume_write(&mut w)?;
    let private_len = w.at - at;
    w.b[4..8].copy_from_slice(&(private_len as u32).to_le_bytes());
    let prefs = app.save().encode().ok_or(Error::Inconsistent)?;
    2u16.put(&mut w)?;
    1u16.put(&mut w)?;
    (prefs.len() as u32).put(&mut w)?;
    w.bytes(&prefs)?;
    let length = w.at;
    let crc = persistence::crc32(&w.b[..length]);
    let mut h = Writer {
        b: &mut out[..HEADER],
        at: 0,
        skip_before: 0,
        stop_after: None,
    };
    h.bytes(b"SBRS")?;
    1u16.put(&mut h)?;
    (HEADER as u16).put(&mut h)?;
    SCHEMA.put(&mut h)?;
    (length as u32).put(&mut h)?;
    sequence.put(&mut h)?;
    app.resume_now().put(&mut h)?;
    3u32.put(&mut h)?;
    0u32.put(&mut h)?;
    0u32.put(&mut h)?;
    crc.put(&mut h)?;
    let crc = persistence::crc32(&h.b[..44]);
    crc.put(&mut h)?;
    Ok(HEADER + length)
}
/// Decode ONLY into a separately initialized candidate. No active app is mutated on failure.
/// The caller commits the validated candidate after success, then enforces the returned neutral gate.
pub fn restore_candidate(
    candidate: &mut App,
    packet: &[u8],
    reboot_clock: u32,
    enabled: bool,
) -> Result<Restored> {
    if !enabled {
        return Err(Error::Disabled);
    }
    // Unbooted storage image is the only legal candidate; prevent accidental live-game mutation.
    if candidate.game.shoe_number != 0 {
        return Err(Error::Inconsistent);
    }
    if packet.len() < HEADER || packet.len() > MAX_BYTES {
        return Err(Error::Truncated);
    }
    let mut r = Reader { b: packet, at: 0 };
    if r.bytes(4)? != b"SBRS"
        || u16::get(&mut r)? != 1
        || u16::get(&mut r)? as usize != HEADER
        || u32::get(&mut r)? != SCHEMA
    {
        return Err(Error::Schema);
    }
    let len = u32::get(&mut r)? as usize;
    let seq = u64::get(&mut r)?;
    let clock = u32::get(&mut r)?;
    if u32::get(&mut r)? != 3 || u32::get(&mut r)? != 0 || u32::get(&mut r)? != 0 {
        return Err(Error::Invalid);
    }
    let payloadcrc = u32::get(&mut r)?;
    let headercrc = u32::get(&mut r)?;
    if len != packet.len() - HEADER
        || headercrc != persistence::crc32(&packet[..44])
        || payloadcrc != persistence::crc32(&packet[HEADER..])
    {
        return Err(Error::Corrupt);
    }
    let mut private = false;
    let mut prefs = None;
    while r.at < packet.len() {
        let tag = u16::get(&mut r)?;
        let flags = u16::get(&mut r)?;
        let len = u32::get(&mut r)? as usize;
        let data = r.bytes(len)?;
        match tag {
            1 => {
                if private || flags != 1 {
                    return Err(Error::Invalid);
                }
                let mut fields = Reader { b: data, at: 0 };
                candidate.resume_read(&mut fields)?;
                if fields.at != data.len() {
                    return Err(Error::Invalid);
                }
                private = true
            }
            2 => {
                if prefs.is_some() || flags != 1 {
                    return Err(Error::Invalid);
                }
                prefs = Some(persistence::Save::decode(data).ok_or(Error::Invalid)?);
            }
            _ => {
                if flags & 1 != 0 {
                    return Err(Error::Schema);
                }
            }
        }
    }
    if let Some(prefs) = prefs {
        candidate.apply_mirrored_preferences(prefs);
    }
    if !private
        || prefs != Some(candidate.save())
        || candidate.resume_now() != clock
        || !candidate.resume_valid()
    {
        return Err(Error::Inconsistent);
    }
    candidate.resume_rebase(reboot_clock);
    candidate.render();
    Ok(Restored {
        sequence: seq,
        saved_clock: clock,
        neutral: NeutralGate { ready: false },
    })
}

/// Cooperative encoder over an immutable App copy in a distinct owned PSRAM region.
/// Service audio/input/USB before each step. Private output is not valid until Done.
/// Source fields are immutable during capture; no expensive semantic re-grading occurs here.
pub struct CaptureTask<'a> {
    app: &'a App,
    out: &'a mut [u8],
    sequence: u64,
    stage: u8,
    cursor: usize,
    private_len: usize,
    total: usize,
    crc: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CaptureProgress {
    More,
    Done { bytes: usize },
}
impl<'a> CaptureTask<'a> {
    pub fn new(app: &'a App, out: &'a mut [u8], sequence: u64, enabled: bool) -> Result<Self> {
        if !enabled {
            return Err(Error::Disabled);
        }
        if out.len() < HEADER + 16 || out.len() > MAX_BYTES {
            return Err(Error::Capacity);
        }
        if app.game.shoe_number == 0 || !app.save().valid() {
            return Err(Error::Inconsistent);
        }
        Ok(Self {
            app,
            out,
            sequence,
            stage: 0,
            cursor: 0,
            private_len: 0,
            total: 0,
            crc: !0,
        })
    }
    pub fn step(&mut self, budget: usize) -> Result<CaptureProgress> {
        let budget = budget.clamp(1, 512);
        match self.stage {
            0 => {
                let mut w = Writer {
                    b: &mut self.out[HEADER + 8..],
                    at: 0,
                    skip_before: self.cursor,
                    stop_after: Some(self.cursor + budget),
                };
                match self.app.resume_write(&mut w) {
                    Err(Error::Yield) => {
                        self.cursor = w.at;
                    }
                    Err(e) => {
                        self.stage = 255;
                        return Err(e);
                    }
                    Ok(()) => {
                        self.private_len = w.at;
                        self.out[HEADER..HEADER + 2].copy_from_slice(&1u16.to_le_bytes());
                        self.out[HEADER + 2..HEADER + 4].copy_from_slice(&1u16.to_le_bytes());
                        self.out[HEADER + 4..HEADER + 8]
                            .copy_from_slice(&(self.private_len as u32).to_le_bytes());
                        self.stage = 1;
                        self.cursor = 0;
                    }
                }
            }
            1 => {
                let prefs = self.app.save().encode().ok_or(Error::Inconsistent)?;
                let at = HEADER + 8 + self.private_len;
                let total = at.checked_add(8 + prefs.len()).ok_or(Error::Capacity)?;
                if total > self.out.len() {
                    return Err(Error::Capacity);
                }
                self.out[at..at + 2].copy_from_slice(&2u16.to_le_bytes());
                self.out[at + 2..at + 4].copy_from_slice(&1u16.to_le_bytes());
                self.out[at + 4..at + 8].copy_from_slice(&(prefs.len() as u32).to_le_bytes());
                let end = (self.cursor + budget).min(prefs.len());
                self.out[at + 8 + self.cursor..at + 8 + end]
                    .copy_from_slice(&prefs[self.cursor..end]);
                self.cursor = end;
                if end == prefs.len() {
                    self.stage = 2;
                    self.cursor = HEADER;
                    self.total = total;
                }
            }
            2 => {
                let end = (self.cursor + budget).min(self.total);
                for &b in &self.out[self.cursor..end] {
                    self.crc ^= b as u32;
                    for _ in 0..8 {
                        self.crc = (self.crc >> 1) ^ if self.crc & 1 != 0 { 0xedb88320 } else { 0 }
                    }
                }
                self.cursor = end;
                if end == self.total {
                    let mut h = Writer {
                        b: &mut self.out[..HEADER],
                        at: 0,
                        skip_before: 0,
                        stop_after: None,
                    };
                    h.bytes(b"SBRS")?;
                    1u16.put(&mut h)?;
                    (HEADER as u16).put(&mut h)?;
                    SCHEMA.put(&mut h)?;
                    ((self.total - HEADER) as u32).put(&mut h)?;
                    self.sequence.put(&mut h)?;
                    self.app.resume_now().put(&mut h)?;
                    3u32.put(&mut h)?;
                    0u32.put(&mut h)?;
                    0u32.put(&mut h)?;
                    (!self.crc).put(&mut h)?;
                    let c = persistence::crc32(&h.b[..44]);
                    c.put(&mut h)?;
                    self.stage = 3;
                    return Ok(CaptureProgress::Done { bytes: self.total });
                }
            }
            3 => return Ok(CaptureProgress::Done { bytes: self.total }),
            _ => return Err(Error::Invalid),
        }
        Ok(CaptureProgress::More)
    }
}

/// Safe fixed-data snapshot copying, with no whole-App return temporary.
pub(crate) trait SnapshotCopy {
    fn snapshot_copy_to(&self, out: &mut Self);
}
impl<T: Copy> SnapshotCopy for T {
    fn snapshot_copy_to(&self, out: &mut Self) {
        *out = *self;
    }
}
