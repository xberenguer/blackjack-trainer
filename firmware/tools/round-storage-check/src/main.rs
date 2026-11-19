//! Actual Waveshare storage adapter on a bounded host NOR stand-in.
//! Synthetic factory-map fixtures only; no device, serial or user data.
#![allow(dead_code)]
extern crate self as esp_storage;
use std::{cell::RefCell, marker::PhantomData, rc::Rc};
pub struct Image {
    bytes: Vec<u8>,
    writes: usize,
    touched: Vec<(u32, usize)>,
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
        image.touched.push((start, (end - start) as usize));
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
        image.touched.push((address, bytes.len()));
        Ok(())
    }
}
#[path = "../../../src/flash_chunks.rs"]
mod flash_chunks;
#[path = "../../../src/round_storage_layout.rs"]
mod round_storage_layout;
#[path = "../../../src/storage.rs"]
mod storage;
#[path = "../../../src/storage_format.rs"]
mod storage_format;
fn main() {}

#[cfg(test)]
mod integration {
    use super::*;
    use private_resume_journal_prototype::Flash as _;
    use stardust_blackjack::persistence::Save;
    fn image_fixture() -> Rc<RefCell<Image>> {
        let mut bytes = vec![0x5a; 0x2000000];
        bytes[0x8000..0x9000].fill(255);
        for (index, (kind, subtype, at, len, name)) in [
            (1, 2, 0x9000u32, 0x32000u32, "nvsfactory"),
            (1, 2, 0x3b000, 0xd2000, "nvs"),
            (1, 0, 0x10d000, 0x2000, "otadata"),
            (1, 1, 0x10f000, 0x1000, "phy_init"),
            (1, 130, 0x110000, 0xf0000, "model"),
            (0, 0, 0x200000, 0x600000, "factory"),
            (1, 130, 0x800000, 0x800000, "storage"),
        ]
        .into_iter()
        .enumerate()
        {
            let row = &mut bytes[0x8000 + index * 32..0x8000 + (index + 1) * 32];
            row.fill(0);
            row[..4].copy_from_slice(&[0xaa, 0x50, kind, subtype]);
            row[4..8].copy_from_slice(&at.to_le_bytes());
            row[8..12].copy_from_slice(&len.to_le_bytes());
            row[12..12 + name.len()].copy_from_slice(name.as_bytes());
        }
        // A bounded ESP32-S3 image layout. Contents are synthetic: the map/extent
        // authorization is tested independently of ROM boot signature checking.
        bytes[0x200000..0x200040].fill(0);
        bytes[0x200000] = 0xe9;
        bytes[0x200001] = 1;
        bytes[0x20000c] = 9;
        bytes[0x20001c..0x200020].copy_from_slice(&16u32.to_le_bytes());
        bytes[0x770000..0x7f2000].fill(255);
        Rc::new(RefCell::new(Image {
            bytes,
            writes: 0,
            touched: vec![],
        }))
    }
    fn storage(image: &Rc<RefCell<Image>>) -> storage::Storage {
        storage::Storage::new(
            Flash {
                image: image.clone(),
                marker: PhantomData,
            },
            [77; 16],
        )
    }
    #[test]
    fn real_adapter_writes_only_the_reserved_journal_tail() {
        let image = image_fixture();
        let before = image.borrow().bytes.clone();
        let mut store = storage(&image);
        assert!(store.load().is_none());
        assert!(store.enabled());
        assert!(!store.imported);
        assert_eq!(store.resume_partition(), Some(0x770000));
        let save = Save::default();
        assert!(store.save(save));
        assert_eq!(store.load().unwrap().encode(), save.encode());
        let mut private = store.private_flash().unwrap();
        assert!(private.erase(0x770000, 4096).is_ok());
        assert!(private.program(0x770000, &[1, 2, 3]).is_ok());
        for address in [
            0, 0x8000, 0x9000, 0x3b000, 0x200000, 0x76f000, 0x7f0000, 0x800000,
        ] {
            assert!(private.erase(address, 4096).is_err());
            assert!(private.program(address, &[0; 4]).is_err());
        }
        drop(private);
        let after = image.borrow();
        assert!(after.writes > 0);
        assert!(
            after
                .touched
                .iter()
                .all(|&(at, len)| round_storage_layout::journal_range(at, len as u32))
        );
        assert_eq!(&after.bytes[..0x770000], &before[..0x770000]);
        assert_eq!(&after.bytes[0x7f2000..], &before[0x7f2000..]);
        drop(after);
        drop(store);
        drop(image);
        // An encrypted/changed factory entry grants no write authority.
        let changed = image_fixture();
        changed.borrow_mut().bytes[0x8000 + 5 * 32 + 28] = 1;
        let mut store = storage(&changed);
        assert!(store.load().is_none());
        assert!(!store.enabled());
        assert!(store.read_failed());
        assert!(!store.save(Save::default()));
        assert!(store.private_flash().is_none());
        assert_eq!(changed.borrow().writes, 0);
        // Likewise, an application extending into the private journal tail.
        let oversized = image_fixture();
        oversized.borrow_mut().bytes[0x20001c..0x200020]
            .copy_from_slice(&round_storage_layout::MAX_APPLICATION_BYTES.to_le_bytes());
        let mut store = storage(&oversized);
        assert!(store.load().is_none());
        assert!(!store.enabled());
        assert!(!store.save(Save::default()));
        assert_eq!(oversized.borrow().writes, 0);
    }
}
