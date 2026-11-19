//! Actual native storage/coordinator running on a bounded host NOR stand-in.
//! Optional STARDUST_OLD_RESUME_READBACK points to a private 0x82000-byte readback;
//! private input bytes/cards/RNG are never printed or copied into release source.
#![allow(dead_code)]
extern crate self as esp_storage;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
pub struct Image {
    bytes: Vec<u8>,
    writes: usize,
}
pub struct Flash<'a> {
    image: Rc<RefCell<Image>>,
    marker: PhantomData<&'a ()>,
}
pub struct FlashStorage<'a> {
    image: Rc<RefCell<Image>>,
    marker: PhantomData<&'a ()>,
}
impl<'a> FlashStorage<'a> {
    pub fn new(flash: Flash<'a>) -> Self {
        Self {
            image: flash.image,
            marker: PhantomData,
        }
    }
    pub fn read_nor(&mut self, address: u32, out: &mut [u8]) -> Result<(), ()> {
        let image = self.image.borrow();
        out.copy_from_slice(
            image
                .bytes
                .get(address as usize..address as usize + out.len())
                .ok_or(())?,
        );
        Ok(())
    }
    pub fn erase(&mut self, start: u32, end: u32) -> Result<(), ()> {
        if start % 4096 != 0 || end % 4096 != 0 {
            return Err(());
        }
        let mut image = self.image.borrow_mut();
        image
            .bytes
            .get_mut(start as usize..end as usize)
            .ok_or(())?
            .fill(255);
        image.writes += 1;
        Ok(())
    }
    pub fn write_nor(&mut self, address: u32, bytes: &[u8]) -> Result<(), ()> {
        if address % 4 != 0 || bytes.len() % 4 != 0 {
            return Err(());
        }
        let mut image = self.image.borrow_mut();
        let out = image
            .bytes
            .get_mut(address as usize..address as usize + bytes.len())
            .ok_or(())?;
        if out.iter().zip(bytes).any(|(old, new)| old & new != *new) {
            return Err(());
        }
        for (old, new) in out.iter_mut().zip(bytes) {
            *old &= new;
        }
        image.writes += 1;
        Ok(())
    }
}
#[path = "../../../src/flash_chunks.rs"]
mod flash_chunks;
#[path = "../../../src/legacy_nvs.rs"]
mod legacy_nvs;
#[path = "../../../src/resume.rs"]
mod resume;
#[path = "../../../src/storage.rs"]
mod storage;
#[path = "../../../src/storage_format.rs"]
mod storage_format;
fn main() {}
#[cfg(test)]
mod tests {
    use super::*;
    use stardust_blackjack::{
        app::App,
        controls::{Event, Key},
        persistence::Save,
        resume_private,
    };
    static TEMPLATE: App = App::EMPTY;
    fn image() -> Rc<RefCell<Image>> {
        let mut bytes = vec![255; 0x800000];
        // Exact production partition positions and types; no hardware dependency.
        for (i, (subtype, start, size, name)) in [
            (2, 0x9000u32, 0x5000u32, "nvs"),
            (0x41, 0x770000, 0x80000, "rustresume"),
            (0x40, 0x7f0000, 0x2000, "rustsave"),
        ]
        .into_iter()
        .enumerate()
        {
            let e = &mut bytes[0x8000 + i * 32..0x8000 + (i + 1) * 32];
            e.fill(0);
            e[..4].copy_from_slice(&[0xaa, 0x50, 1, subtype]);
            e[4..8].copy_from_slice(&start.to_le_bytes());
            e[8..12].copy_from_slice(&size.to_le_bytes());
            e[12..12 + name.len()].copy_from_slice(name.as_bytes());
        }
        Rc::new(RefCell::new(Image { bytes, writes: 0 }))
    }
    fn storage(image: &Rc<RefCell<Image>>) -> storage::Storage {
        storage::Storage::new(
            Flash {
                image: image.clone(),
                marker: PhantomData,
            },
            [91; 16],
        )
    }
    fn runtime() -> resume::Resume {
        let frozen = Box::leak(Box::new(App::EMPTY));
        let candidate = Box::leak(Box::new(App::EMPTY));
        let private = vec![0; resume_private::MAX_BYTES].leak();
        let bound = vec![0; resume_private::MAX_BYTES].leak();
        let public = vec![0; stardust_blackjack::persistence::SIZE].leak();
        unsafe {
            resume::Resume::new(
                resume::Regions {
                    frozen,
                    candidate,
                    private: private.as_mut_ptr(),
                    bound: bound.as_mut_ptr(),
                    public: public.as_mut_ptr(),
                    template: &TEMPLATE,
                },
                [92; 16],
            )
        }
    }
    #[test]
    fn raw_legacy_public_is_preserved_until_new_commit() {
        let image = image();
        let mut old = Save::default().encode().unwrap();
        old[4..6].copy_from_slice(&9u16.to_le_bytes());
        old[202] = 0;
        old[203] = 0;
        old[583] = 1;
        let crc = stardust_blackjack::persistence::crc32(&old[..old.len() - 4]);
        let end = old.len();
        old[end - 4..].copy_from_slice(&crc.to_le_bytes());
        let meta = storage_format::Meta {
            generation: 7,
            epoch: 8,
            nonce: [9; 16],
        };
        let record = storage_format::record_v2(&old, meta).unwrap();
        image.borrow_mut().bytes[0x7f0000..0x7f0000 + record.len]
            .copy_from_slice(&record.bytes[..record.len]);
        let mut store = storage(&image);
        let save = store.load().unwrap();
        assert_eq!(store.committed().unwrap().packet, old);
        assert_eq!(store.committed().unwrap().generation, 7);
        assert_eq!(image.borrow().writes, 0);
        assert_ne!(
            save.encode().unwrap(),
            old,
            "migration must change version/defaults"
        );
        assert!(store.checkpoint(save));
        assert_eq!(store.committed().unwrap().generation, 8);
        assert_eq!(store.committed().unwrap().packet, save.encode().unwrap());
        assert_eq!(
            &image.borrow().bytes[0x7f0000..0x7f0000 + record.len],
            &record.bytes[..record.len],
            "old committed slot retained"
        );
        // Actual predecessor checkpoint regression is optional private input.
        if let Some(path) = std::env::var_os("STARDUST_OLD_RESUME_READBACK") {
            actual_checkpoint(path);
        }
    }
    fn actual_checkpoint(path: std::ffi::OsString) {
        let readback = std::fs::read(path).expect("old private+public readback readable");
        assert_eq!(readback.len(), 0x82000);
        let image = image();
        image.borrow_mut().bytes[0x770000..0x7f2000].copy_from_slice(&readback);
        let mut store = storage(&image);
        let save = store.load().expect("actual legacy public load");
        assert!(save.resume_enabled);
        let raw = store.committed().unwrap().packet.to_vec();
        let generation = store.committed().unwrap().generation;
        let mut app = Box::leak(Box::new(App::new(321)));
        assert!(app.load(save));
        let mut r = runtime();
        r.boot(app, &mut store, 1);
        assert!(matches!(
            r.chooser.state,
            stardust_blackjack::resume_ui::BootState::ChooseSaved { .. }
        ));
        for now in [2, 1000, 10000] {
            r.step(app, &mut store, now);
            assert_eq!(image.borrow().writes, 0);
            assert_eq!(store.committed().unwrap().packet, raw);
        }
        r.chooser_events(&mut app, &mut store, [false, false], &[], 10001);
        r.chooser_events(
            &mut app,
            &mut store,
            [false, false],
            &[Event::Tap(Key::Blue)],
            10002,
        );
        assert!(!r.choosing());
        assert!(
            !r.outputs_allowed(app),
            "canonical migration ACK must precede frame/audio/status"
        );
        r.step(app, &mut store, 20000);
        assert_eq!(
            image.borrow().writes,
            0,
            "no private job before public migration ACK"
        );
        app.render();
        assert!(r.acknowledge(app, &mut store, 20001));
        assert!(r.outputs_allowed(app));
        assert_eq!(
            store.committed().unwrap().generation,
            generation.wrapping_add(1)
        );
        assert_eq!(
            store.committed().unwrap().packet,
            app.save().encode().unwrap()
        );
        assert_eq!(app.view_name(), "CountAnswer");
        assert_eq!(app.training.stats().all.decisions, 194);
        assert_eq!(
            &image.borrow().bytes[0x770000..0x7f0000],
            &readback[..0x80000],
            "private checkpoint untouched until future cooperative capture"
        );
        println!(
            "actual V9 checkpoint: raw identity preserved through chooser; exact pending-question restore; V10 migration ACK before outputs/private; lifetime totals retained"
        );
    }
}
