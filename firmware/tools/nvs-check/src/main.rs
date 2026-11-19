#[path = "../../../src/legacy_nvs.rs"]
mod legacy_nvs;
#[path = "../../../src/storage_format.rs"]
mod storage_format;
fn main() {
    let path = std::env::args_os()
        .nth(1)
        .expect("usage: stardust-nvs-check NVS.bin|rustsave.bin");
    let raw = std::fs::read(path).expect("read saved partition");
    let result = match raw.len() {
        0x5000 => {
            let mut page = [0; 4096];
            legacy_nvs::import(raw.len() as u32, &mut page, |i, p| {
                if let Some(b) = raw.get(i * 4096..(i + 1) * 4096) {
                    p.copy_from_slice(b);
                    true
                } else {
                    false
                }
            })
        }
        0x2000 => {
            let n = storage_format::JOURNAL_BYTES;
            storage_format::choose_slots(Ok(&raw[..n]), Ok(&raw[4096..4096 + n]))
                .ok()
                .flatten()
                .map(|chosen| {
                    println!(
                        "source=rustsave slot={} sequence={} current={}",
                        chosen.slot, chosen.sequence, chosen.current
                    );
                    chosen.save
                })
        }
        _ => panic!("exact original NVS20480 or rustsave8192 bytes required"),
    };
    if let Some(s) = result {
        println!(
            "valid=true mode={:?} brightness={} sound={} flipped={} profile={:?} penetration={} min={} max={} count={} guide_count={} decisions={} correct={} errors={} assisted={} count_decisions={} count_correct={} extra_seats={} player_seat={} wager_enabled={} wager_decisions={}",
            s.mode,
            s.brightness,
            s.sound,
            s.flipped,
            s.profile,
            s.penetration,
            s.minimum_bet,
            s.maximum_bet,
            s.count_display,
            s.guide_count_display,
            s.stats.all.decisions,
            s.stats.all.correct,
            s.stats.all.errors,
            s.stats.all.assisted,
            s.count_stats.all.decisions,
            s.count_stats.all.correct,
            s.extra_seats,
            s.player_seat,
            s.wager_enabled,
            s.wager_stats.all.decisions
        );
    } else {
        println!("valid=false");
        std::process::exit(2)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use stardust_blackjack::persistence::Save;
    fn entry(ns: u8, ty: u8, span: u8, chunk: u8, key: &[u8], value: [u8; 8]) -> [u8; 32] {
        let mut e = [255; 32];
        e[..4].copy_from_slice(&[ns, ty, span, chunk]);
        e[8..24].fill(0);
        e[8..8 + key.len()].copy_from_slice(key);
        e[24..].copy_from_slice(&value);
        let mut data = [0; 28];
        data[..4].copy_from_slice(&e[..4]);
        data[4..].copy_from_slice(&e[8..]);
        e[4..8].copy_from_slice(&storage_format::nvs_crc32(&data).to_le_bytes());
        e
    }
    fn fixture(version: u8) -> Vec<u8> {
        let current = Save::default().encode().unwrap();
        let mut packet = [0; 336];
        packet[..332].copy_from_slice(&current[..332]);
        packet[4..6].copy_from_slice(&5u16.to_le_bytes());
        packet[6..8].copy_from_slice(&336u16.to_le_bytes());
        // V5 predates seat/audio settings. Their old reserved bytes must stay zero;
        // current V10 defaults write volume/speed here and are not valid V5 bytes.
        packet[200..204].fill(0);
        let c = storage_format::crc32(&packet[..332]);
        packet[332..].copy_from_slice(&c.to_le_bytes());
        let mut raw = vec![255; 0x5000];
        let p = &mut raw[..4096];
        p[..4].copy_from_slice(&0xfffffffeu32.to_le_bytes());
        p[4..8].copy_from_slice(&0u32.to_le_bytes());
        p[8] = version;
        let c = storage_format::nvs_crc32(&p[4..28]);
        p[28..32].copy_from_slice(&c.to_le_bytes());
        let mut v = [255; 8];
        v[0] = 3;
        p[64..96].copy_from_slice(&entry(0, 1, 1, 255, b"stardustbj", v));
        let mut v = [255; 8];
        v[..2].copy_from_slice(&336u16.to_le_bytes());
        v[4..8].copy_from_slice(&storage_format::nvs_crc32(&packet).to_le_bytes());
        p[96..128].copy_from_slice(&entry(
            3,
            if version == 0xff { 0x41 } else { 0x42 },
            12,
            if version == 0xff { 255 } else { 128 },
            b"userdata",
            v,
        ));
        p[128..464].copy_from_slice(&packet);
        let mut count = 13;
        if version != 0xff {
            let mut v = [255; 8];
            v[..4].copy_from_slice(&336u32.to_le_bytes());
            v[4] = 1;
            v[5] = 128;
            p[64 + 13 * 32..96 + 13 * 32].copy_from_slice(&entry(3, 0x48, 1, 255, b"userdata", v));
            count = 14;
        }
        for i in 0..count {
            p[32 + i / 4] &= !(1 << (2 * (i % 4)));
        }
        raw
    }
    fn parse(raw: &[u8]) -> Option<Save> {
        let mut p = [0; 4096];
        legacy_nvs::import(raw.len() as u32, &mut p, |i, p| {
            p.copy_from_slice(&raw[i * 4096..(i + 1) * 4096]);
            true
        })
    }
    #[test]
    fn both_nvs_layouts_import_exact_save() {
        for v in [0xff, 0xfe] {
            assert_eq!(parse(&fixture(v)), Some(Save::default()));
        }
    }
    #[test]
    fn corrupt_headers_payload_commit_are_rejected() {
        let raw = fixture(0xfe);
        for i in [
            4,
            8,
            28,
            64,
            68,
            88,
            96,
            100,
            120,
            128,
            400,
            64 + 13 * 32,
            68 + 13 * 32,
        ] {
            let mut b = raw.clone();
            b[i] ^= 1;
            assert!(parse(&b).is_none(), "offset{i}");
        }
        let mut b = raw;
        b[32 + 13 / 4] |= 1 << (2 * (13 % 4));
        assert!(parse(&b).is_none());
    }
    #[test]
    fn journal_and_legacy_crc_are_deliberately_different() {
        assert_ne!(
            storage_format::crc32(b"SBTR"),
            storage_format::nvs_crc32(b"SBTR")
        );
    }
}
#[cfg(test)]
mod legacy_v5_encoder;
#[cfg(test)]
mod journal_tests {
    use super::*;
    use stardust_blackjack::persistence::Save;
    const V5: &[u8; 336] = include_bytes!("../../../../tests/data/save-v5.bin");
    #[test]
    fn authentic_v5_record_migrates_every_field() {
        let old = legacy_v5_encoder::original_record(V5, 79);
        let expected = Save::decode(V5).unwrap();
        let chosen =
            storage_format::choose_slots(Ok(&old), Ok(&[255; storage_format::JOURNAL_BYTES]))
                .unwrap()
                .unwrap();
        assert_eq!(chosen.save, expected);
        assert_eq!(chosen.sequence, 79);
        assert!(!chosen.current);
        let new = expected.encode().unwrap();
        assert_eq!(new.len(), stardust_blackjack::persistence::SIZE);
        let record = storage_format::record(&new, 80).unwrap();
        let chosen = storage_format::choose_slots(Ok(&old), Ok(&record.bytes))
            .unwrap()
            .unwrap();
        assert!(chosen.current);
        assert_eq!(chosen.save, expected);
        assert_eq!(chosen.sequence, 80);
        assert_eq!(chosen.slot, 1);
    }
    #[test]
    fn newest_rust_record_is_authoritative_over_stale_legacy_save() {
        let old = legacy_v5_encoder::original_record(V5, 79);
        let mut latest = Save::decode(V5).unwrap();
        latest.brightness = 255;
        latest.sound = true;
        let bytes = latest.encode().unwrap();
        let new = storage_format::record(&bytes, 80).unwrap();
        let chosen = storage_format::choose_slots(Ok(&old), Ok(&new.bytes))
            .unwrap()
            .unwrap();
        assert_eq!(chosen.save, latest);
        assert_ne!(chosen.save, Save::decode(V5).unwrap());
    }
    #[test]
    fn journal_read_error_is_not_empty_or_migration_permission() {
        let old = legacy_v5_encoder::original_record(V5, 79);
        assert!(storage_format::choose_slots(Ok(&old), Err(())).is_err());
        assert!(storage_format::choose_slots(Err(()), Ok(&old)).is_err());
        assert!(storage_format::choose_slots(Err(()), Err(())).is_err());
    }
    #[test]
    fn every_v7_program_interruption_preserves_original_v5() {
        let old = legacy_v5_encoder::original_record(V5, 79);
        let expected = Save::decode(V5).unwrap();
        let mut settings = expected;
        settings.brightness = 255;
        let new = storage_format::record(&settings.encode().unwrap(), 80).unwrap();
        for cut in 0..=new.len {
            let mut partial = [255; storage_format::JOURNAL_BYTES];
            partial[..cut].copy_from_slice(&new.bytes[..cut]);
            let chosen = storage_format::choose_slots(Ok(&old), Ok(&partial))
                .unwrap()
                .unwrap();
            assert_eq!(
                chosen.save,
                if cut == new.len { settings } else { expected }
            );
        }
        for byte in 0..new.len {
            for bit in 0..8 {
                let mut bad = new.bytes;
                bad[byte] ^= 1 << bit;
                let chosen = storage_format::choose_slots(Ok(&old), Ok(&bad))
                    .unwrap()
                    .unwrap();
                assert_eq!(chosen.save, expected);
            }
        }
    }
}
#[cfg(test)]
mod v7_migration {
    use super::*;
    use stardust_blackjack::persistence::Save;
    #[test]
    fn v7_preferences_and_large_wager_counters_survive_envelope_migration() {
        let old = include_bytes!("../fixtures/save-v7-synthetic.bin");
        let expected = Save::decode(old).unwrap();
        assert_eq!(expected.extra_seats, 3);
        assert_eq!(expected.player_seat, 2);
        assert!(expected.wager_enabled);
        assert_eq!(expected.wager_policy.units, [1, 3, 5, 9, 13]);
        assert!(expected.wager_stats.all.decisions > (1u64 << 53));
        let record = storage_format::record(old, 123).unwrap();
        let chosen = storage_format::choose_slots(
            Ok(&record.bytes),
            Ok(&[255; storage_format::JOURNAL_BYTES]),
        )
        .unwrap()
        .unwrap();
        assert_eq!(chosen.save, expected);
        let current = storage_format::record(&chosen.save.encode().unwrap(), 124).unwrap();
        let migrated = storage_format::choose_slots(Ok(&record.bytes), Ok(&current.bytes))
            .unwrap()
            .unwrap();
        assert_eq!(migrated.save, expected);
        assert!(migrated.current);
        assert_eq!(migrated.sequence, 124);
    }
}
#[path = "../../../src/flash_chunks.rs"]
mod flash_chunks;
#[cfg(test)]
mod public_v2_tests {
    use super::storage_format as f;
    use stardust_blackjack::persistence::Save;
    #[test]
    fn public_v2_every_cut_bit_wrap_and_v1_migration() {
        let packet = Save::default().encode().unwrap();
        let meta = f::Meta {
            generation: u64::MAX,
            epoch: 91,
            nonce: [17; 16],
        };
        let old = f::record(&packet, u32::MAX).unwrap();
        let next = f::record_v2(&packet, meta).unwrap();
        assert_eq!(f::decode_v2(&next.bytes).unwrap(), (meta, &packet[..]));
        for cut in 0..next.len {
            let mut torn = vec![255; 4096];
            torn[..cut].copy_from_slice(&next.bytes[..cut]);
            let chosen = f::choose_slots(Ok(&old.bytes), Ok(&torn)).unwrap().unwrap();
            assert!(chosen.meta.is_none());
        }
        for i in 0..next.len {
            for bit in 0..8 {
                let mut bad = next.bytes;
                bad[i] ^= 1 << bit;
                assert!(f::decode_v2(&bad).is_none());
            }
        }
        let chosen = f::choose_slots(Ok(&old.bytes), Ok(&next.bytes))
            .unwrap()
            .unwrap();
        assert_eq!(chosen.meta, Some(meta));
        let newer = f::record_v2(
            &packet,
            f::Meta {
                generation: 0,
                epoch: 92,
                nonce: [17; 16],
            },
        )
        .unwrap();
        assert_eq!(
            f::choose_slots(Ok(&next.bytes), Ok(&newer.bytes))
                .unwrap()
                .unwrap()
                .meta
                .unwrap()
                .generation,
            0
        );
        assert!(f::choose_slots(Err(()), Ok(&next.bytes)).is_err());
    }
}
#[cfg(test)]
mod committed_corruption_tests {
    use super::storage_format as f;
    use stardust_blackjack::{
        app::App, persistence::Save, resume_binding as b, resume_private as s,
    };
    #[test]
    fn every_new_committed_public_bit_damage_refuses_old_private_identity() {
        let mut a = App::new(7);
        a.resume_enabled = true;
        a.render();
        let old_packet = a.save().encode().unwrap();
        let old_meta = f::Meta {
            generation: 4,
            epoch: 9,
            nonce: [13; 16],
        };
        let old = f::record_v2(&old_packet, old_meta).unwrap();
        let mut private = vec![0; s::MAX_BYTES];
        let n = s::capture_private(&a, &mut private, 9, true).unwrap();
        let public = b::PublicCommit {
            generation: 4,
            checkpoint_epoch: 9,
            transaction_session: [13; 16],
            packet: &old_packet,
        };
        let mut bundle = vec![0; s::MAX_BYTES];
        let n = b::bind_private(&mut bundle, public, &private[..n]).unwrap();
        let mut latest = Save::default();
        latest.resume_enabled = true;
        latest.stats.all.decisions = 1;
        latest.stats.all.errors = 1;
        latest.stats.categories[0] = latest.stats.all;
        let latest_packet = latest.encode().unwrap();
        let newer = f::record_v2(
            &latest_packet,
            f::Meta {
                generation: 5,
                epoch: 10,
                nonce: [13; 16],
            },
        )
        .unwrap();
        let mut c = App::EMPTY;
        assert!(
            b::restore_bound(&mut c, &bundle[..n], public, 0, true).is_ok(),
            "old binding alone is insufficient without resume_safe gate"
        );
        for at in 0..newer.len {
            for bit in 0..8 {
                let mut corrupt = newer.bytes;
                corrupt[at] ^= 1 << bit;
                let chosen = f::choose_slots(Ok(&old.bytes), Ok(&corrupt))
                    .unwrap()
                    .unwrap();
                assert_eq!(chosen.save, a.save());
                assert!(
                    !chosen.resume_safe,
                    "committed corruption byte{at}bit{bit} must not permit old private snapshot"
                );
            }
        }
    }
    #[test]
    fn interrupted_new_public_header_keeps_old_scores_but_forces_fresh_resume() {
        let packet = Save::default().encode().unwrap();
        let old = f::record_v2(
            &packet,
            f::Meta {
                generation: u64::MAX,
                epoch: 7,
                nonce: [7; 16],
            },
        )
        .unwrap();
        let next = f::record_v2(
            &packet,
            f::Meta {
                generation: 0,
                epoch: 8,
                nonce: [7; 16],
            },
        )
        .unwrap();
        let mut torn = vec![255; 4096];
        torn[..64].copy_from_slice(&next.bytes[..64]);
        let chosen = f::choose_slots(Ok(&old.bytes), Ok(&torn)).unwrap().unwrap();
        assert_eq!(chosen.save, Save::default());
        assert!(!chosen.resume_safe);
        assert_eq!(chosen.generation_floor, 0);
        assert_eq!(chosen.epoch_floor, 8);
    }
}
