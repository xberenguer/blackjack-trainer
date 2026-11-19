#![no_std]
#![forbid(unsafe_code)]
//! Private full-resume journal prototype, no USB/export API or hardware access.
pub const HEADER: usize = 40;
pub const FOOTER: usize = 32;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Error {
    Io,
    Bounds,
    Corrupt,
    Disabled,
}
/// Hardware implementations must stage PSRAM-backed reads/programs through aligned internal-RAM chunks while caches are disabled.
pub trait Flash {
    fn read(&mut self, offset: usize, out: &mut [u8]) -> Result<(), Error>;
    fn erase(&mut self, offset: usize, len: usize) -> Result<(), Error>;
    fn program(&mut self, offset: usize, bytes: &[u8]) -> Result<(), Error>;
}
pub fn crc(bytes: &[u8]) -> u32 {
    let mut c = !0;
    for &b in bytes {
        c ^= b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    !c
}
fn u32at(b: &[u8], p: usize) -> u32 {
    u32::from_le_bytes(b[p..p + 4].try_into().unwrap())
}
fn u64at(b: &[u8], p: usize) -> u64 {
    u64::from_le_bytes(b[p..p + 8].try_into().unwrap())
}
fn newer(a: u64, b: u64) -> bool {
    (a.wrapping_sub(b) as i64) > 0
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Record {
    pub slot: usize,
    pub sequence: u64,
    pub session: u64,
    pub length: usize,
    pub tombstone: bool,
}
pub struct Journal<const SLOT: usize> {
    base: usize,
}
impl<const SLOT: usize> Journal<SLOT> {
    pub const fn new(base: usize) -> Self {
        Self { base }
    }
    fn slot(&self, n: usize) -> Result<usize, Error> {
        if n >= 2 || SLOT < 4096 || !SLOT.is_multiple_of(4096) {
            return Err(Error::Bounds);
        }
        self.base
            .checked_add(n.checked_mul(SLOT).ok_or(Error::Bounds)?)
            .ok_or(Error::Bounds)
    }
    fn inspect(
        &self,
        flash: &mut impl Flash,
        slot: usize,
        scratch: &mut [u8],
    ) -> Result<Record, Error> {
        let base = self.slot(slot)?;
        let mut h = [0; HEADER];
        let mut f = [0; FOOTER];
        flash.read(base, &mut h)?;
        flash.read(base + SLOT - FOOTER, &mut f)?;
        if &h[..4] != b"RSMJ"
            || u32at(&h, 4) != 1
            || u32at(&h, 36) != crc(&h[..36])
            || &f[..8] != b"COMMIT01"
            || u64at(&f, 8) != u64at(&h, 8)
            || u64at(&f, 16) != u64at(&h, 16)
            || u32at(&f, 24) != u32at(&h, 36)
            || u32at(&f, 28) != crc(&f[..28])
        {
            return Err(Error::Corrupt);
        }
        let length = u32at(&h, 24) as usize;
        let flags = u32at(&h, 32);
        if length > SLOT - HEADER - FOOTER
            || length > scratch.len()
            || flags > 1
            || (flags == 1 && length != 0)
        {
            return Err(Error::Bounds);
        }
        flash.read(base + HEADER, &mut scratch[..length])?;
        if u32at(&h, 28) != crc(&scratch[..length]) {
            return Err(Error::Corrupt);
        }
        Ok(Record {
            slot,
            sequence: u64at(&h, 8),
            session: u64at(&h, 16),
            length,
            tombstone: flags == 1,
        })
    }
    /// Caller owns scratch large enough for one bounded private snapshot; hardware Flash stages external-memory buffers internally.
    pub fn latest(&self, flash: &mut impl Flash, scratch: &mut [u8]) -> Result<Record, Error> {
        let a = match self.inspect(flash, 0, scratch) {
            Ok(r) => Some(r),
            Err(Error::Io) => return Err(Error::Io),
            Err(_) => None,
        };
        let b = match self.inspect(flash, 1, scratch) {
            Ok(r) => Some(r),
            Err(Error::Io) => return Err(Error::Io),
            Err(_) => None,
        };
        let chosen = match (a, b) {
            (Some(a), Some(b)) => {
                if newer(b.sequence, a.sequence) {
                    b
                } else {
                    a
                }
            }
            (Some(a), _) => a,
            (_, Some(b)) => b,
            _ => return Err(Error::Corrupt),
        };
        self.inspect(flash, chosen.slot, scratch)
    }
    pub fn commit(
        &self,
        flash: &mut impl Flash,
        session: u64,
        payload: &[u8],
        enabled: bool,
        scratch: &mut [u8],
    ) -> Result<Record, Error> {
        if !enabled {
            return Err(Error::Disabled);
        }
        self.write(flash, session, payload, false, scratch)
    }
    pub fn invalidate(
        &self,
        flash: &mut impl Flash,
        session: u64,
        scratch: &mut [u8],
    ) -> Result<Record, Error> {
        self.write(flash, session, &[], true, scratch)
    }
    fn write(
        &self,
        flash: &mut impl Flash,
        session: u64,
        payload: &[u8],
        tombstone: bool,
        scratch: &mut [u8],
    ) -> Result<Record, Error> {
        if payload.len() > SLOT - HEADER - FOOTER
            || payload.len() > scratch.len()
            || payload.len() > u32::MAX as usize
        {
            return Err(Error::Bounds);
        }
        let old = match self.latest(flash, scratch) {
            Ok(r) => Some(r),
            Err(Error::Corrupt) => None,
            Err(e) => return Err(e),
        };
        let slot = old.map_or(0, |r| 1 - r.slot);
        let sequence = old.map_or(1, |r| r.sequence.wrapping_add(1));
        let base = self.slot(slot)?;
        let mut h = [0; HEADER];
        h[..4].copy_from_slice(b"RSMJ");
        h[4..8].copy_from_slice(&1u32.to_le_bytes());
        h[8..16].copy_from_slice(&sequence.to_le_bytes());
        h[16..24].copy_from_slice(&session.to_le_bytes());
        h[24..28].copy_from_slice(&(payload.len() as u32).to_le_bytes());
        h[28..32].copy_from_slice(&crc(payload).to_le_bytes());
        h[32..36].copy_from_slice(&(tombstone as u32).to_le_bytes());
        let c = crc(&h[..36]);
        h[36..40].copy_from_slice(&c.to_le_bytes());
        let mut f = [0; FOOTER];
        f[..8].copy_from_slice(b"COMMIT01");
        f[8..16].copy_from_slice(&sequence.to_le_bytes());
        f[16..24].copy_from_slice(&session.to_le_bytes());
        f[24..28].copy_from_slice(&c.to_le_bytes());
        let c = crc(&f[..28]);
        f[28..32].copy_from_slice(&c.to_le_bytes());
        // Old slot is never touched. Interrupting any erase/program step leaves it valid.
        flash.erase(base, SLOT)?;
        flash.program(base, &h)?;
        for (offset, chunk) in payload.chunks(512).enumerate() {
            flash.program(base + HEADER + offset * 512, chunk)?;
        }
        flash.program(base + SLOT - FOOTER, &f)?;
        let r = self.inspect(flash, slot, scratch)?;
        if scratch[..payload.len()] != *payload {
            return Err(Error::Corrupt);
        }
        Ok(r)
    }
}
#[cfg(test)]
extern crate std;
#[cfg(test)]
mod tests {
    use super::*;
    use std::vec::Vec;
    #[derive(Clone)]
    pub(super) struct Model {
        pub(super) bytes: Vec<u8>,
        pub(super) budget: Option<usize>,
        pub(super) changed: usize,
        read_fail: Option<usize>,
    }
    impl Model {
        pub(super) fn new(n: usize) -> Self {
            Self {
                bytes: std::vec![255;n],
                budget: None,
                changed: 0,
                read_fail: None,
            }
        }
        fn mutate(&mut self, offset: usize, value: u8) -> Result<(), Error> {
            if let Some(b) = self.budget.as_mut() {
                if *b == 0 {
                    return Err(Error::Io);
                }
                *b -= 1
            }
            self.bytes[offset] = value;
            self.changed += 1;
            Ok(())
        }
    }
    impl Flash for Model {
        fn read(&mut self, p: usize, b: &mut [u8]) -> Result<(), Error> {
            if self.read_fail == Some(p) {
                return Err(Error::Io);
            }
            b.copy_from_slice(self.bytes.get(p..p + b.len()).ok_or(Error::Bounds)?);
            Ok(())
        }
        fn erase(&mut self, p: usize, n: usize) -> Result<(), Error> {
            if p % 4096 != 0 || n % 4096 != 0 || p + n > self.bytes.len() {
                return Err(Error::Bounds);
            }
            for i in p..p + n {
                self.mutate(i, 255)?
            }
            Ok(())
        }
        fn program(&mut self, p: usize, b: &[u8]) -> Result<(), Error> {
            if p + b.len() > self.bytes.len() {
                return Err(Error::Bounds);
            }
            for (i, &v) in b.iter().enumerate() {
                let old = self.bytes[p + i];
                if v | old != old {
                    return Err(Error::Io);
                }
                self.mutate(p + i, old & v)?
            }
            Ok(())
        }
    }
    #[test]
    fn every_byte_power_loss_keeps_old_or_complete_new() {
        let j = Journal::<4096>::new(0);
        let mut f = Model::new(8192);
        let mut scratch = [0; 2048];
        let old = [0x35; 1024];
        let new = [0xa7; 2048];
        j.commit(&mut f, 7, &old, true, &mut scratch).unwrap();
        let mut success = f.clone();
        success.changed = 0;
        j.commit(&mut success, 7, &new, true, &mut scratch).unwrap();
        let operations = success.changed;
        for cut in 0..=operations {
            let mut interrupted = f.clone();
            interrupted.budget = Some(cut);
            let result = j.commit(&mut interrupted, 7, &new, true, &mut scratch);
            interrupted.budget = None;
            let r = j.latest(&mut interrupted, &mut scratch).unwrap();
            assert!(r.sequence == 1 || r.sequence == 2);
            if r.sequence == 1 {
                assert_eq!(&scratch[..r.length], &old)
            } else {
                assert_eq!(&scratch[..r.length], &new)
            }
            if result.is_ok() {
                assert_eq!(r.sequence, 2)
            }
        }
        std::println!("exhaustive interruption points={}", operations + 1);
    }
    #[test]
    fn every_bit_corruption_of_current_record_falls_back() {
        let j = Journal::<4096>::new(0);
        let mut f = Model::new(8192);
        let mut scratch = [0; 256];
        j.commit(&mut f, 9, &[3; 256], true, &mut scratch).unwrap();
        j.commit(&mut f, 9, &[7; 256], true, &mut scratch).unwrap();
        for p in (4096..4096 + HEADER + 256).chain(8192 - FOOTER..8192) {
            for bit in 0..8 {
                let mut damaged = f.clone();
                damaged.bytes[p] ^= 1 << bit;
                let r = j.latest(&mut damaged, &mut scratch).unwrap();
                assert_eq!(r.sequence, 1);
                assert_eq!(scratch, [3; 256]);
            }
        }
    }
    #[test]
    fn opt_in_and_tombstone_prevent_resurrection() {
        let j = Journal::<4096>::new(0);
        let mut f = Model::new(8192);
        let mut scratch = [0; 256];
        let unchanged = f.bytes.clone();
        assert_eq!(
            j.commit(&mut f, 1, b"private", false, &mut scratch),
            Err(Error::Disabled)
        );
        assert_eq!(f.bytes, unchanged);
        j.commit(&mut f, 1, b"private", true, &mut scratch).unwrap();
        j.invalidate(&mut f, 2, &mut scratch).unwrap();
        let r = j.latest(&mut f, &mut scratch).unwrap();
        assert!(r.tombstone);
        assert_eq!(r.session, 2);
        assert_eq!(r.length, 0);
    }
    #[test]
    fn bounds_and_generation_wrap() {
        assert!(newer(0, u64::MAX));
        assert!(!newer(u64::MAX, 0));
        let j = Journal::<4096>::new(0);
        let mut f = Model::new(8192);
        let mut scratch = [0; 4096];
        assert_eq!(
            j.commit(&mut f, 1, &[0; 4096], true, &mut scratch),
            Err(Error::Bounds)
        );
        assert!(f.bytes.iter().all(|&b| b == 255));
    }
    #[test]
    fn read_failure_blocks_commit_before_erase() {
        let j = Journal::<4096>::new(0);
        let mut f = Model::new(8192);
        let mut scratch = [0; 256];
        j.commit(&mut f, 1, b"first", true, &mut scratch).unwrap();
        j.commit(&mut f, 1, b"latest", true, &mut scratch).unwrap();
        for address in [0, 4096, 4096 - FOOTER, 8192 - FOOTER, HEADER, 4096 + HEADER] {
            let mut broken = f.clone();
            broken.read_fail = Some(address);
            let before = broken.bytes.clone();
            assert_eq!(
                j.commit(&mut broken, 1, b"next", true, &mut scratch),
                Err(Error::Io)
            );
            assert_eq!(broken.bytes, before);
        }
    }
}

/// Cooperative writer: call only after servicing buttons/audio/USB. Each call hashes<=512B,
/// erases exactly one4KB page, programs<=512B, or verifies<=512B. No hidden data is logged.
pub struct CommitTask<'a, const SLOT: usize> {
    base: usize,
    payload: &'a [u8],
    record: Record,
    header: [u8; HEADER],
    footer: [u8; FOOTER],
    stage: u8,
    at: usize,
    crc_state: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Progress {
    More,
    Committed(Record),
}
impl<'a, const SLOT: usize> CommitTask<'a, SLOT> {
    /// `previous` comes only from a successfully validated boot scan or prior commit cache.
    pub fn new(
        journal: &Journal<SLOT>,
        previous: Option<Record>,
        session: u64,
        payload: &'a [u8],
        enabled: bool,
    ) -> Result<Self, Error> {
        if !enabled {
            return Err(Error::Disabled);
        }
        if SLOT < 4096
            || !SLOT.is_multiple_of(4096)
            || payload.len() > SLOT - HEADER - FOOTER
            || payload.len() > u32::MAX as usize
        {
            return Err(Error::Bounds);
        }
        let slot = previous.map_or(0, |r| 1 - r.slot);
        let base = journal.slot(slot)?;
        let sequence = previous.map_or(1, |r| r.sequence.wrapping_add(1));
        Ok(Self {
            base,
            payload,
            record: Record {
                slot,
                sequence,
                session,
                length: payload.len(),
                tombstone: false,
            },
            header: [0; HEADER],
            footer: [0; FOOTER],
            stage: 0,
            at: 0,
            crc_state: !0,
        })
    }
    pub fn tombstone(
        journal: &Journal<SLOT>,
        previous: Option<Record>,
        session: u64,
    ) -> Result<Self, Error> {
        let mut task = Self::new(journal, previous, session, &[], true)?;
        task.record.tombstone = true;
        Ok(task)
    }
    pub fn step(&mut self, flash: &mut impl Flash) -> Result<Progress, Error> {
        let result = self.step_inner(flash);
        if result.is_err() {
            self.stage = 255
        }
        result
    }
    fn step_inner(&mut self, flash: &mut impl Flash) -> Result<Progress, Error> {
        match self.stage {
            0 => {
                let end = (self.at + 512).min(self.payload.len());
                for &b in &self.payload[self.at..end] {
                    self.crc_state ^= b as u32;
                    for _ in 0..8 {
                        self.crc_state = (self.crc_state >> 1)
                            ^ if self.crc_state & 1 != 0 {
                                0xedb88320
                            } else {
                                0
                            }
                    }
                }
                self.at = end;
                if end == self.payload.len() {
                    let h = &mut self.header;
                    h[..4].copy_from_slice(b"RSMJ");
                    h[4..8].copy_from_slice(&1u32.to_le_bytes());
                    h[8..16].copy_from_slice(&self.record.sequence.to_le_bytes());
                    h[16..24].copy_from_slice(&self.record.session.to_le_bytes());
                    h[24..28].copy_from_slice(&(self.payload.len() as u32).to_le_bytes());
                    h[28..32].copy_from_slice(&(!self.crc_state).to_le_bytes());
                    h[32..36].copy_from_slice(&(self.record.tombstone as u32).to_le_bytes());
                    let c = crc(&h[..36]);
                    h[36..40].copy_from_slice(&c.to_le_bytes());
                    let f = &mut self.footer;
                    f[..8].copy_from_slice(b"COMMIT01");
                    f[8..16].copy_from_slice(&self.record.sequence.to_le_bytes());
                    f[16..24].copy_from_slice(&self.record.session.to_le_bytes());
                    f[24..28].copy_from_slice(&c.to_le_bytes());
                    let c = crc(&f[..28]);
                    f[28..32].copy_from_slice(&c.to_le_bytes());
                    self.at = 0;
                    self.stage = 1;
                }
            }
            1 => {
                // Invalidate the old target's fixed commit sector first. Stale bytes beyond
                // declared payload length are never read or hashed, so erase only used pages.
                let body_end = (HEADER + self.payload.len()).div_ceil(4096) * 4096;
                let body_end = body_end.min(SLOT - 4096);
                if self.at == 0 {
                    flash.erase(self.base + SLOT - 4096, 4096)?;
                    self.at = 4096;
                } else {
                    flash.erase(self.base + self.at - 4096, 4096)?;
                    self.at += 4096;
                }
                if self.at - 4096 >= body_end {
                    self.at = 0;
                    self.stage = 2;
                }
            }
            2 => {
                flash.program(self.base, &self.header)?;
                self.stage = 3;
            }
            3 => {
                let end = (self.at + 512).min(self.payload.len());
                if end > self.at {
                    flash.program(self.base + HEADER + self.at, &self.payload[self.at..end])?;
                }
                self.at = end;
                if self.at == self.payload.len() {
                    self.at = 0;
                    self.stage = 4;
                }
            }
            4 => {
                flash.program(self.base + SLOT - FOOTER, &self.footer)?;
                self.stage = 5;
            }
            5 => {
                let mut h = [0; HEADER];
                flash.read(self.base, &mut h)?;
                if h != self.header {
                    return Err(Error::Corrupt);
                }
                self.stage = 6;
            }
            6 => {
                let end = (self.at + 512).min(self.payload.len());
                let mut b = [0; 512];
                let n = end - self.at;
                if n > 0 {
                    flash.read(self.base + HEADER + self.at, &mut b[..n])?;
                    if b[..n] != self.payload[self.at..end] {
                        return Err(Error::Corrupt);
                    }
                }
                self.at = end;
                if end == self.payload.len() {
                    self.stage = 7;
                }
            }
            7 => {
                let mut f = [0; FOOTER];
                flash.read(self.base + SLOT - FOOTER, &mut f)?;
                if f != self.footer {
                    return Err(Error::Corrupt);
                }
                self.stage = 8;
                return Ok(Progress::Committed(self.record));
            }
            8 => return Ok(Progress::Committed(self.record)),
            _ => return Err(Error::Io),
        }
        Ok(Progress::More)
    }
}
#[cfg(test)]
mod cooperative_tests {
    use super::tests::Model;
    use super::*;
    struct Bounded<'a> {
        inner: &'a mut Model,
        calls: usize,
    }
    impl Flash for Bounded<'_> {
        fn read(&mut self, p: usize, b: &mut [u8]) -> Result<(), Error> {
            assert!(b.len() <= 512);
            self.calls += 1;
            self.inner.read(p, b)
        }
        fn erase(&mut self, p: usize, n: usize) -> Result<(), Error> {
            assert_eq!(n, 4096);
            self.calls += 1;
            self.inner.erase(p, n)
        }
        fn program(&mut self, p: usize, b: &[u8]) -> Result<(), Error> {
            assert!(b.len() <= 512);
            self.calls += 1;
            self.inner.program(p, b)
        }
    }
    #[test]
    fn cooperative_and_sync_records_are_identical_and_each_step_is_bounded() {
        let j = Journal::<8192>::new(0);
        let mut f = Model::new(16384);
        let mut scratch = [0; 6000];
        let old = j.commit(&mut f, 7, b"old", true, &mut scratch).unwrap();
        let payload = [0xa7; 6000];
        let mut sync = f.clone();
        let expected = j
            .commit(&mut sync, 7, &payload, true, &mut scratch)
            .unwrap();
        let mut task = CommitTask::new(&j, Some(old), 7, &payload, true).unwrap();
        let mut steps = 0;
        loop {
            let mut bounded = Bounded {
                inner: &mut f,
                calls: 0,
            };
            let result = task.step(&mut bounded).unwrap();
            assert!(bounded.calls <= 1);
            steps += 1;
            if let Progress::Committed(record) = result {
                assert_eq!(record, expected);
                break;
            }
        }
        assert_eq!(f.bytes, sync.bytes);
        std::println!("cooperative journal steps={steps}");
    }
    #[test]
    fn tombstones_are_cooperative_and_match_sync_invalidation() {
        let j = Journal::<8192>::new(0);
        let mut f = Model::new(16384);
        let mut scratch = [0; 256];
        let prior = j.commit(&mut f, 7, b"old", true, &mut scratch).unwrap();
        let mut sync = f.clone();
        let expected = j.invalidate(&mut sync, 8, &mut scratch).unwrap();
        let mut task = CommitTask::tombstone(&j, Some(prior), 8).unwrap();
        loop {
            let mut b = Bounded {
                inner: &mut f,
                calls: 0,
            };
            let p = task.step(&mut b).unwrap();
            assert!(b.calls <= 1);
            if let Progress::Committed(r) = p {
                assert_eq!(r, expected);
                assert!(r.tombstone);
                break;
            }
        }
        assert_eq!(f.bytes, sync.bytes);
    }
}

#[cfg(test)]
mod sparse_fault_tests {
    use super::tests::Model;
    use super::*;
    #[test]
    fn every_sparse_erase_program_byte_cut_keeps_latest_or_complete_new() {
        let j = Journal::<32768>::new(0);
        let mut base = Model::new(65536);
        let mut scratch = [0; 20000];
        j.commit(&mut base, 1, &[0x31; 18000], true, &mut scratch)
            .unwrap();
        let previous = j
            .commit(&mut base, 1, b"latest-other-slot", true, &mut scratch)
            .unwrap();
        let payload = [0x75; 4503];
        let mut complete = base.clone();
        complete.changed = 0;
        let mut task = CommitTask::new(&j, Some(previous), 1, &payload, true).unwrap();
        loop {
            if let Progress::Committed(_) = task.step(&mut complete).unwrap() {
                break;
            }
        }
        let operations = complete.changed;
        assert_eq!(
            &base.bytes[8192..28672],
            &complete.bytes[8192..28672],
            "unused stale tail stays untouched"
        );
        assert_eq!(
            &base.bytes[32768..],
            &complete.bytes[32768..],
            "latest slot stays untouched"
        );
        assert_eq!(operations, 3 * 4096 + HEADER + payload.len() + FOOTER);
        for cut in 0..=operations {
            let mut f = base.clone();
            f.budget = Some(cut);
            let mut task = CommitTask::new(&j, Some(previous), 1, &payload, true).unwrap();
            loop {
                match task.step(&mut f) {
                    Ok(Progress::More) => {}
                    Ok(Progress::Committed(_)) | Err(Error::Io) => break,
                    other => panic!("unexpected{other:?}"),
                }
            }
            f.budget = None;
            let record = j.latest(&mut f, &mut scratch).unwrap();
            if record.sequence == previous.sequence {
                assert_eq!(&scratch[..record.length], b"latest-other-slot")
            } else {
                assert_eq!(record.sequence, previous.sequence + 1);
                assert_eq!(&scratch[..record.length], &payload);
            }
        }
        std::println!(
            "sparse_exact_byte_cuts={} erased_pages=3 slot_pages=8 stale_tail_preserved=true",
            operations + 1
        );
    }
}
