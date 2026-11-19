//! NVS is read-only. Writes are confined to the explicitly declared rustsave partition.
use crate::storage_format::{self as fmt, JOURNAL_BYTES, PACKET};
use esp_storage::{Flash, FlashStorage};
use stardust_blackjack::persistence::Save;
#[repr(align(4))]
struct Aligned<const N: usize>([u8; N]);
const SCRATCH_BYTES: usize = if JOURNAL_BYTES * 2 > 4096 { 8192 } else { 4096 };
static mut SCRATCH: Aligned<SCRATCH_BYTES> = Aligned([0; SCRATCH_BYTES]);
pub struct Storage {
    resume_safe: bool,
    generation_floor: u64,
    epoch_floor: u64,
    meta: Option<fmt::Meta>,
    nonce: [u8; 16],
    private_base: Option<usize>,
    flash: FlashStorage<'static>,
    base: Option<u32>,
    sequence: u32,
    slot: usize,
    pub imported: bool,
    pub writes: u32,
    pub errors: u32,
    last: [u8; PACKET],
    read_failed: bool,
}
impl Storage {
    pub fn new(flash: Flash<'static>, nonce: [u8; 16]) -> Self {
        Self {
            resume_safe: true,
            generation_floor: 0,
            epoch_floor: 0,
            meta: None,
            nonce,
            private_base: None,
            flash: FlashStorage::new(flash),
            base: None,
            sequence: 0,
            slot: 1,
            imported: false,
            writes: 0,
            errors: 0,
            last: [0; PACKET],
            read_failed: false,
        }
    }
    fn scratch() -> &'static mut [u8; SCRATCH_BYTES] {
        // SAFETY: single main-core owner; every use ends before another storage operation.
        unsafe { &mut (*core::ptr::addr_of_mut!(SCRATCH)).0 }
    }
    pub fn load(&mut self) -> Option<Save> {
        self.base = None;
        self.resume_safe = true;
        self.generation_floor = 0;
        self.epoch_floor = 0;
        self.meta = None;
        self.private_base = None;
        self.read_failed = false;
        let b = &mut Self::scratch()[..4096];
        if self.flash.read_nor(0x8000, b).is_err() {
            self.read_failed = true;
            self.errors += 1;
            return None;
        }
        // Reject overlapping/overflowing/nonstandard partitions before enabling any write.
        let mut ends = [(0u32, 0u32); 96];
        let mut count = 0;
        let mut nvs = None;
        let mut journal = None;
        let mut private = None;
        for e in b.chunks_exact(32) {
            if e[..2] == [255, 255] || e[..2] == [0xeb, 0xeb] {
                break;
            }
            if e[..2] != [0xaa, 0x50] || count == 96 {
                return None;
            }
            let start = u32::from_le_bytes(e[4..8].try_into().ok()?);
            let size = u32::from_le_bytes(e[8..12].try_into().ok()?);
            let end = start.checked_add(size)?;
            if start < 0x9000 || end > 0x800000 || size == 0 {
                return None;
            }
            for &(a, z) in &ends[..count] {
                if start < z && a < end {
                    return None;
                }
            }
            ends[count] = (start, end);
            count += 1;
            if e[12..16] == *b"nvs\0" && e[2] == 1 && e[3] == 2 && start == 0x9000 && size == 0x5000
            {
                nvs = Some((start, size));
            }
            if e[12..23] == *b"rustresume\0"
                && e[2] == 1
                && e[3] == 0x41
                && start == 0x770000
                && size == 0x80000
                && e[28..32] == [0; 4]
            {
                private = Some(start as usize);
            }
            if e[12..21] == *b"rustsave\0"
                && e[2] == 1
                && e[3] == 0x40
                && start == 0x7f0000
                && size == 0x2000
                && e[28..32] == [0; 4]
            {
                journal = Some(start);
            }
        }
        self.base = journal;
        self.private_base = private;
        if let Some(base) = journal {
            let b = Self::scratch();
            let (first, second) = b[..JOURNAL_BYTES * 2].split_at_mut(JOURNAL_BYTES);
            let first_ok = self.flash.read_nor(base, first).is_ok();
            let second_ok = self.flash.read_nor(base + 4096, second).is_ok();
            let chosen = fmt::choose_slots(
                if first_ok { Ok(first) } else { Err(()) },
                if second_ok { Ok(second) } else { Err(()) },
            );
            match chosen {
                Err(()) => {
                    self.read_failed = true;
                    self.base = None;
                    self.errors += 1;
                    return None;
                }
                Ok(Some(chosen)) => {
                    self.resume_safe = chosen.resume_safe;
                    self.generation_floor = chosen.generation_floor;
                    self.epoch_floor = chosen.epoch_floor;
                    self.meta = chosen.meta;
                    self.sequence = chosen.sequence;
                    self.slot = chosen.slot;
                    self.last = if chosen.meta.is_some() {
                        // The transaction identity binds the exact stored bytes.
                        // Decoding V9 preferences into V10 defaults must not
                        // silently replace that packet before the boot choice.
                        let selected = if chosen.slot == 0 { &*first } else { &*second };
                        fmt::decode_v2(selected)?.1.try_into().ok()?
                    } else {
                        [0; PACKET]
                    };
                    return Some(chosen.save);
                }
                Ok(None) => {}
            }
        }

        let save = nvs.and_then(|(start, size)| self.import_nvs(start, size));
        if let Some(s) = save {
            self.imported = true;
            return Some(s);
        }
        None
    }
    fn import_nvs(&mut self, start: u32, size: u32) -> Option<Save> {
        let mut failed = false;
        let result = crate::legacy_nvs::import(
            size,
            (&mut Self::scratch()[..4096])
                .try_into()
                .expect("flash sector"),
            |page, out| {
                if self
                    .flash
                    .read_nor(start + page as u32 * 4096, out)
                    .is_err()
                {
                    failed = true;
                    false
                } else {
                    true
                }
            },
        );
        if failed {
            self.read_failed = true;
            self.base = None;
            self.errors += 1;
        }
        result
    }
    pub fn save(&mut self, save: Save) -> bool {
        let Some(packet) = save.encode() else {
            return false;
        };
        if packet == self.last {
            return true;
        }
        self.commit_packet(packet)
    }
    /// Force new public identity even for byte-identical scoreless gameplay. Call before displaying changes.
    pub fn checkpoint(&mut self, save: Save) -> bool {
        let Some(packet) = save.encode() else {
            return false;
        };
        self.commit_packet(packet)
    }
    fn commit_packet(&mut self, packet: [u8; PACKET]) -> bool {
        let Some(base) = self.base else { return false };
        if self.read_failed {
            return false;
        }
        let slot = 1 - self.slot;
        let target = base + slot as u32 * 4096;
        let meta = fmt::Meta {
            generation: self.generation_floor.wrapping_add(1),
            epoch: self.epoch_floor.wrapping_add(1),
            nonce: self.nonce,
        };
        let Some(record) = fmt::record_v2(&packet, meta) else {
            return false;
        };
        let len = record.len;
        let commit = len - 4;
        let b = Self::scratch();
        b[..len].copy_from_slice(&record.bytes[..len]);
        if self.flash.erase(target, target + 4096).is_err()
            || self.flash.write_nor(target, &b[..commit]).is_err()
            || self
                .flash
                .write_nor(target + commit as u32, &b[commit..len])
                .is_err()
        {
            self.errors += 1;
            return false;
        }
        b[..JOURNAL_BYTES].fill(0);
        if self
            .flash
            .read_nor(target, &mut b[..JOURNAL_BYTES])
            .is_err()
            || fmt::decode_v2(&b[..JOURNAL_BYTES])
                .map(|(m, p)| m == meta && p == packet)
                .is_none_or(|valid| !valid)
        {
            self.errors += 1;
            self.read_failed = true;
            self.base = None;
            return false;
        }
        self.last = packet;
        self.meta = Some(meta);
        self.resume_safe = true;
        self.generation_floor = meta.generation;
        self.epoch_floor = meta.epoch;
        self.sequence = meta.generation as u32;
        self.slot = slot;
        self.writes += 1;
        true
    }
    pub fn resume_safe(&self) -> bool {
        self.resume_safe
    }
    pub fn committed(&self) -> Option<stardust_blackjack::resume_binding::PublicCommit<'_>> {
        if !self.resume_safe {
            return None;
        }
        let m = self.meta?;
        Some(stardust_blackjack::resume_binding::PublicCommit {
            generation: m.generation,
            checkpoint_epoch: m.epoch,
            transaction_session: m.nonce,
            packet: &self.last,
        })
    }
    pub fn resume_partition(&self) -> Option<usize> {
        self.private_base
    }
    pub fn rotate_nonce(&mut self, nonce: [u8; 16]) {
        self.nonce = nonce;
    }
    pub fn private_flash(&mut self) -> Option<PrivateFlash<'_>> {
        let base = self.private_base?;
        if self.read_failed {
            return None;
        }
        Some(PrivateFlash {
            flash: &mut self.flash,
            base,
        })
    }
    pub fn read_failed(&self) -> bool {
        self.read_failed
    }
    pub fn enabled(&self) -> bool {
        self.base.is_some() && !self.read_failed
    }
}

#[repr(align(4))]
struct PrivateScratch(crate::flash_chunks::Scratch);
static mut PRIVATE_SCRATCH: PrivateScratch = PrivateScratch(crate::flash_chunks::Scratch([0; 512]));
pub struct PrivateFlash<'a> {
    flash: &'a mut FlashStorage<'static>,
    base: usize,
}
impl PrivateFlash<'_> {
    fn bounds(&self, p: usize, n: usize) -> Result<(), private_resume_journal_prototype::Error> {
        if p < self.base || p.checked_add(n).is_none_or(|end| end > self.base + 0x80000) {
            Err(private_resume_journal_prototype::Error::Bounds)
        } else {
            Ok(())
        }
    }
    fn scratch() -> &'static mut crate::flash_chunks::Scratch {
        // SAFETY: the single main-core storage owner completes each synchronous driver operation before another staging use. DMA does not reference this buffer.
        unsafe { &mut (*core::ptr::addr_of_mut!(PRIVATE_SCRATCH)).0 }
    }
}
impl private_resume_journal_prototype::Flash for PrivateFlash<'_> {
    fn read(
        &mut self,
        p: usize,
        out: &mut [u8],
    ) -> Result<(), private_resume_journal_prototype::Error> {
        self.bounds(p, out.len().div_ceil(4) * 4)?;
        crate::flash_chunks::read(p, out, Self::scratch(), |p, b| {
            self.flash
                .read_nor(p as u32, b)
                .map_err(|_| private_resume_journal_prototype::Error::Io)
        })
    }
    fn program(
        &mut self,
        p: usize,
        b: &[u8],
    ) -> Result<(), private_resume_journal_prototype::Error> {
        self.bounds(p, b.len().div_ceil(4) * 4)?;
        crate::flash_chunks::program(p, b, Self::scratch(), |p, b| {
            self.flash
                .write_nor(p as u32, b)
                .map_err(|_| private_resume_journal_prototype::Error::Io)
        })
    }
    fn erase(&mut self, p: usize, n: usize) -> Result<(), private_resume_journal_prototype::Error> {
        self.bounds(p, n)?;
        if p % 4096 != 0 || n != 4096 {
            return Err(private_resume_journal_prototype::Error::Bounds);
        }
        self.flash
            .erase(p as u32, (p + n) as u32)
            .map_err(|_| private_resume_journal_prototype::Error::Io)
    }
}
