//! Exact little-endian SBTR V10 packets; V1–V9 migration preserves uint64 scores.
//! Session/shoe/history are deliberately absent at this stage of the roadmap.
use crate::{
    count_practice,
    rules::{Profile, Rules},
    training::{self, Counters, Mode, Policy},
};
pub const VERSION: u16 = 10;
pub const SIZE: usize = 2636;
pub const V7_SIZE: usize = 584;
pub const V5_SIZE: usize = 336;
pub const PREVIOUS_SIZE: usize = 208;
pub const LEGACY_SIZE: usize = 196;
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Save {
    pub resume_enabled: bool,
    pub drill_target: u8,
    pub drill_mix: u8,
    pub drill_profile: Profile,
    pub drill_stats: crate::deviation_drills::Stats,
    pub extra_seats: u8,
    pub player_seat: u8,
    pub wager_enabled: bool,
    pub wager_policy: crate::wager_practice::Policy,
    pub wager_stats: crate::wager_practice::Stats,
    pub mode: Mode,
    pub brightness: u8,
    pub sound: bool,
    pub audio_volume: u8,
    pub audio_mask: u8,
    pub fast_mode: bool,
    pub flipped: bool,
    pub pause_on_mistake: bool,
    pub profile: Profile,
    pub penetration: u8,
    pub count_display: u8,
    pub guide_count_display: u8,
    pub minimum_bet: u32,
    pub maximum_bet: u32,
    pub stats: training::Stats,
    pub policy: Policy,
    pub count_stats: count_practice::Stats,
}
impl Default for Save {
    fn default() -> Self {
        Self {
            resume_enabled: false,
            drill_target: 31,
            drill_mix: 0,
            drill_profile: Profile::SixH17,
            drill_stats: crate::deviation_drills::Stats::EMPTY,
            extra_seats: 0,
            player_seat: 0,
            wager_enabled: false,
            wager_policy: crate::wager_practice::Policy::EXAMPLE,
            wager_stats: crate::wager_practice::Stats::EMPTY,
            mode: Mode::Off,
            brightness: 255,
            sound: true,
            audio_volume: 20,
            audio_mask: 7,
            fast_mode: true,
            flipped: false,
            pause_on_mistake: false,
            profile: Profile::SixH17,
            penetration: 75,
            count_display: 0,
            guide_count_display: 2,
            minimum_bet: 500,
            maximum_bet: 50000,
            stats: training::Stats::default(),
            policy: Policy::HiLo,
            count_stats: count_practice::Stats::default(),
        }
    }
}
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut c = !0u32;
    for b in bytes {
        c ^= *b as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ if c & 1 != 0 { 0xedb88320 } else { 0 }
        }
    }
    !c
}
fn u16_at(b: &[u8], p: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(p..p + 2)?.try_into().ok()?))
}
fn u32_at(b: &[u8], p: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(p..p + 4)?.try_into().ok()?))
}
fn read64(b: &[u8], p: &mut usize) -> Option<u64> {
    let n = u64::from_le_bytes(b.get(*p..*p + 8)?.try_into().ok()?);
    *p += 8;
    Some(n)
}
fn read_counters(b: &[u8], p: &mut usize) -> Option<Counters> {
    Some(Counters {
        decisions: read64(b, p)?,
        correct: read64(b, p)?,
        errors: read64(b, p)?,
        assisted: read64(b, p)?,
    })
}
fn write64(b: &mut [u8], p: &mut usize, n: u64) {
    b[*p..*p + 8].copy_from_slice(&n.to_le_bytes());
    *p += 8;
}
impl Save {
    pub fn rules(self) -> Rules {
        let mut r = Rules::for_profile(self.profile);
        r.minimum = self.minimum_bet as i64;
        r.maximum = self.maximum_bet as i64;
        r.penetration = self.penetration;
        r
    }
    pub fn valid(self) -> bool {
        self.audio_volume <= 100
            && self.audio_mask & !7 == 0
            && self.drill_target <= 31
            && self.drill_mix <= 3
            && self.drill_stats.valid()
            && self.extra_seats <= 6
            && self.player_seat <= self.extra_seats
            && self.wager_policy.valid()
            && self.wager_stats.valid()
            && self.brightness >= 20
            && self.count_display <= 2
            && self.guide_count_display <= 2
            && self.rules().valid()
            && self.stats.valid()
            && self.count_stats.valid()
    }
    pub fn encode(self) -> Option<[u8; SIZE]> {
        if !self.valid() {
            return None;
        }
        let mut out = [0u8; SIZE];
        out[..4].copy_from_slice(b"SBTR");
        out[4..6].copy_from_slice(&VERSION.to_le_bytes());
        out[6..8].copy_from_slice(&(SIZE as u16).to_le_bytes());
        out[8] = self.mode as u8;
        out[9] = self.brightness;
        out[10] = u8::from(self.sound)
            | (u8::from(self.flipped) << 1)
            | (u8::from(self.pause_on_mistake) << 2);
        out[11] = self.profile as u8;
        out[12] = self.penetration;
        out[13] = self.count_display;
        out[14] = self.guide_count_display;
        out[15] = self.policy as u8;
        out[16..20].copy_from_slice(&self.minimum_bet.to_le_bytes());
        out[20..24].copy_from_slice(&self.maximum_bet.to_le_bytes());
        let s = self.stats;
        let mut p = 24;
        for n in [
            s.all.decisions,
            s.all.correct,
            s.all.errors,
            s.all.assisted,
            s.streak,
            s.best_streak,
        ] {
            write64(&mut out, &mut p, n)
        }
        for c in s.categories {
            for n in [c.decisions, c.correct, c.errors, c.assisted] {
                write64(&mut out, &mut p, n)
            }
        }
        p = 204;
        for c in [
            self.count_stats.all,
            self.count_stats.kinds[0],
            self.count_stats.kinds[1],
            self.count_stats.kinds[2],
        ] {
            for n in [c.decisions, c.correct, c.errors, c.assisted] {
                write64(&mut out, &mut p, n)
            }
        }
        out[200] = self.extra_seats;
        out[201] = self.player_seat;
        out[202] = self.audio_volume;
        out[203] = self.audio_mask | (u8::from(self.fast_mode) << 3);
        out[332] = u8::from(self.wager_enabled);
        out[333] = self.wager_policy.version as u8;
        for (i, n) in self.wager_policy.units.into_iter().enumerate() {
            out[336 + i * 4..340 + i * 4].copy_from_slice(&n.to_le_bytes());
        }
        p = 356;
        for c in core::iter::once(self.wager_stats.all).chain(self.wager_stats.categories) {
            for n in [c.decisions, c.correct, c.errors, c.assisted] {
                write64(&mut out, &mut p, n);
            }
        }
        out[580] = self.drill_target;
        out[581] = self.drill_mix;
        out[582] = self.drill_profile as u8;
        out[583] = u8::from(self.resume_enabled);
        p = 584;
        for totals in [self.drill_stats.first, self.drill_stats.replay] {
            for c in core::iter::once(totals.all).chain(totals.indices) {
                for n in [c.decisions, c.correct, c.errors, c.assisted] {
                    write64(&mut out, &mut p, n)
                }
            }
        }
        let crc = crc32(&out[..SIZE - 4]);
        out[SIZE - 4..].copy_from_slice(&crc.to_le_bytes());
        Some(out)
    }
    pub fn decode(b: &[u8]) -> Option<Self> {
        if ![LEGACY_SIZE, PREVIOUS_SIZE, V5_SIZE, V7_SIZE, SIZE].contains(&b.len())
            || b.get(..4)? != b"SBTR"
        {
            return None;
        }
        let version = u16_at(b, 4)?;
        let expected = match version {
            1 | 2 => LEGACY_SIZE,
            3 | 4 => PREVIOUS_SIZE,
            5 | 6 => V5_SIZE,
            7 => V7_SIZE,
            8..=10 => SIZE,
            _ => return None,
        };
        if b.len() != expected
            || u16_at(b, 6)? as usize != expected
            || u32_at(b, expected - 4)? != crc32(&b[..expected - 4])
            || b[9] < 20
            || b[10] & 0xf8 != 0
            || b[8] > if version == 1 { 2 } else { 3 }
        {
            return None;
        }
        if version < 3 {
            if b[11..16].iter().any(|v| *v != 0) {
                return None;
            }
        } else if (version < 6 && b[200..204].iter().any(|v| *v != 0))
            || ((6..10).contains(&version) && b[202..204].iter().any(|v| *v != 0))
            || (version >= 10 && (b[202] > 100 || b[203] & 0xf0 != 0))
            || (version < 5 && b[15] != 0)
            || (version == 3 && b[14] != 0)
        {
            return None;
        }
        let mut result = Self {
            // V1–V4 predate policy selection and used Basic.
            policy: Policy::Basic,
            mode: Mode::from_id(b[8])?,
            brightness: b[9],
            sound: b[10] & 1 != 0,
            flipped: b[10] & 2 != 0,
            pause_on_mistake: b[10] & 4 != 0,
            ..Self::default()
        };
        if version >= 3 {
            result.profile = Profile::from_id(b[11])?;
            result.penetration = b[12];
            result.count_display = b[13];
            result.minimum_bet = u32_at(b, 16)?;
            result.maximum_bet = u32_at(b, 20)?;
        }
        if version >= 4 {
            result.guide_count_display = b[14];
        }
        if version >= 5 {
            result.policy = match b[15] {
                0 => Policy::Basic,
                1 => Policy::HiLo,
                _ => return None,
            };
        }
        let mut p = if version < 3 { 16 } else { 24 };
        result.stats.all = read_counters(b, &mut p)?;
        result.stats.streak = read64(b, &mut p)?;
        result.stats.best_streak = read64(b, &mut p)?;
        for c in &mut result.stats.categories {
            *c = read_counters(b, &mut p)?;
        }
        if version >= 5 {
            p = 204;
            result.count_stats.all = read_counters(b, &mut p)?;
            for c in &mut result.count_stats.kinds {
                *c = read_counters(b, &mut p)?;
            }
        }
        if version >= 6 {
            result.extra_seats = b[200];
            result.player_seat = b[201];
        }
        if version >= 10 {
            result.audio_volume = b[202];
            result.audio_mask = b[203] & 7;
            result.fast_mode = b[203] & 8 != 0;
        }
        if version >= 7 {
            if b[332] > 1 || b[333] != 0 || b[334..336].iter().any(|&n| n != 0) {
                return None;
            }
            result.wager_enabled = b[332] != 0;
            for (i, n) in result.wager_policy.units.iter_mut().enumerate() {
                *n = u32_at(b, 336 + i * 4)?;
            }
            p = 356;
            result.wager_stats.all = read_counters(b, &mut p)?;
            for c in &mut result.wager_stats.categories {
                *c = read_counters(b, &mut p)?;
            }
        }
        if version >= 8 {
            result.resume_enabled = version >= 9 && b[583] == 1;
            if b[580] > 31 || b[581] > 3 || b[583] > if version >= 9 { 1 } else { 0 } {
                return None;
            }
            result.drill_target = b[580];
            result.drill_mix = b[581];
            result.drill_profile = Profile::from_id(b[582])?;
            p = 584;
            result.drill_stats.first.all = read_counters(b, &mut p)?;
            for c in &mut result.drill_stats.first.indices {
                *c = read_counters(b, &mut p)?
            }
            result.drill_stats.replay.all = read_counters(b, &mut p)?;
            for c in &mut result.drill_stats.replay.indices {
                *c = read_counters(b, &mut p)?
            }
        }
        if result.valid() { Some(result) } else { None }
    }
}
