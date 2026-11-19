//! Exact Waveshare 1.75C factory-map authority. No partition-table, calibration,
//! NVS or filesystem writes are permitted. Blackjack owns only its declared
//! journal tail inside the factory application's otherwise unused space.
pub const FACTORY_START: u32 = 0x200000;
pub const PRIVATE_START: u32 = 0x770000;
pub const PRIVATE_BYTES: u32 = 0x80000;
pub const PUBLIC_START: u32 = 0x7f0000;
pub const PUBLIC_BYTES: u32 = 0x2000;
pub const MAX_APPLICATION_BYTES: u32 = PRIVATE_START - FACTORY_START;
const TABLE_BYTES: usize = 4096;
const ROW_BYTES: usize = 32;
const ROWS: [(u8, u8, u32, u32, &[u8]); 7] = [
    (1, 2, 0x9000, 0x32000, b"nvsfactory"),
    (1, 2, 0x3b000, 0xd2000, b"nvs"),
    (1, 0, 0x10d000, 0x2000, b"otadata"),
    (1, 1, 0x10f000, 0x1000, b"phy_init"),
    (1, 130, 0x110000, 0xf0000, b"model"),
    (0, 0, FACTORY_START, 0x600000, b"factory"),
    (1, 130, 0x800000, 0x800000, b"storage"),
];
// MD5 of the exact seven canonical 32-byte entries above. Exact byte equality
// makes a general hash implementation unnecessary in this authorization gate.
const TABLE_MD5: [u8; 16] = [
    0x59, 0xee, 0x05, 0x69, 0x79, 0x0e, 0xe8, 0x82, 0x4d, 0x2a, 0xdb, 0x10, 0xea, 0xec, 0xfa, 0x40,
];
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Error {
    Table,
    Read,
    Image,
}

/// Accept only the complete unencrypted factory map, optional valid MD5 footer,
/// and erased trailing bytes. A prefix match or plausible factory row is insufficient.
pub fn valid_table(table: &[u8]) -> bool {
    if table.len() != TABLE_BYTES {
        return false;
    }
    for (index, &(kind, subtype, start, bytes, name)) in ROWS.iter().enumerate() {
        let row = &table[index * ROW_BYTES..(index + 1) * ROW_BYTES];
        let mut expected = [0u8; ROW_BYTES];
        expected[..4].copy_from_slice(&[0xaa, 0x50, kind, subtype]);
        expected[4..8].copy_from_slice(&start.to_le_bytes());
        expected[8..12].copy_from_slice(&bytes.to_le_bytes());
        expected[12..12 + name.len()].copy_from_slice(name);
        if row != expected {
            return false;
        }
    }
    let tail = &table[ROWS.len() * ROW_BYTES..];
    if tail.iter().all(|byte| *byte == 0xff) {
        return true;
    }
    tail[..2] == [0xeb, 0xeb]
        && tail[2..16] == [0xff; 14]
        && tail[16..32] == TABLE_MD5
        && tail[32..].iter().all(|byte| *byte == 0xff)
}

#[repr(align(4))]
struct Header<const N: usize>([u8; N]);

/// Parse the running application's bounded ESP image layout before granting any
/// journal authority. The bootloader owns image checksum/SHA verification; this
/// independent extent check prevents a manually built oversized image from
/// erasing its own executable segments through the reserved journal tail.
pub fn application_end(
    mut read: impl FnMut(u32, &mut [u8]) -> Result<(), ()>,
) -> Result<u32, Error> {
    let mut header = Header([0; 24]);
    read(FACTORY_START, &mut header.0).map_err(|_| Error::Read)?;
    if header.0[0] != 0xe9
        || !(1..=16).contains(&header.0[1])
        || header.0[12..14] != [9, 0]
        || header.0[23] > 1
    {
        return Err(Error::Image);
    }
    let mut at = FACTORY_START + 24;
    let mut segment = Header([0; 8]);
    for _ in 0..header.0[1] {
        let after_header = at.checked_add(8).ok_or(Error::Image)?;
        if after_header > PRIVATE_START {
            return Err(Error::Image);
        }
        read(at, &mut segment.0).map_err(|_| Error::Read)?;
        let bytes = u32::from_le_bytes(segment.0[4..8].try_into().map_err(|_| Error::Image)?);
        if bytes == 0 {
            return Err(Error::Image);
        }
        at = after_header.checked_add(bytes).ok_or(Error::Image)?;
        if at > PRIVATE_START {
            return Err(Error::Image);
        }
    }
    // ESP checksum occupies the last byte of a sixteen-byte aligned block.
    // The optional appended image SHA256 follows that checksum block.
    let end = at.checked_add(16).ok_or(Error::Image)? & !15;
    let end = end
        .checked_add(if header.0[23] == 1 { 32 } else { 0 })
        .ok_or(Error::Image)?;
    if end > PRIVATE_START {
        return Err(Error::Image);
    }
    Ok(end)
}

/// Explicit write envelopes for review/tests. Firmware uses these same constants
/// for its public journal and bounded private Flash adapter.
pub fn journal_range(start: u32, bytes: u32) -> bool {
    let Some(end) = start.checked_add(bytes) else {
        return false;
    };
    bytes > 0
        && ((start >= PRIVATE_START && end <= PRIVATE_START + PRIVATE_BYTES)
            || (start >= PUBLIC_START && end <= PUBLIC_START + PUBLIC_BYTES))
}

#[cfg(test)]
mod tests {
    use super::*;
    fn table(md5: bool) -> [u8; TABLE_BYTES] {
        let mut table = [255; TABLE_BYTES];
        for (index, &(kind, subtype, start, bytes, name)) in ROWS.iter().enumerate() {
            let row = &mut table[index * ROW_BYTES..(index + 1) * ROW_BYTES];
            row.fill(0);
            row[..4].copy_from_slice(&[0xaa, 0x50, kind, subtype]);
            row[4..8].copy_from_slice(&start.to_le_bytes());
            row[8..12].copy_from_slice(&bytes.to_le_bytes());
            row[12..12 + name.len()].copy_from_slice(name);
        }
        if md5 {
            table[224..226].copy_from_slice(&[0xeb, 0xeb]);
            table[240..256].copy_from_slice(&TABLE_MD5);
        }
        table
    }
    #[test]
    fn complete_exact_factory_map_only() {
        assert!(valid_table(&table(false)));
        assert!(valid_table(&table(true)));
        assert!(!valid_table(&table(false)[..256]));
        // Every changed byte in every entry (including labels, flags and ranges)
        // loses authority; the original no-footer form remains a valid control.
        for at in 0..224 {
            let mut changed = table(false);
            changed[at] ^= 1;
            assert!(!valid_table(&changed));
        }
        for at in 224..256 {
            let mut changed = table(true);
            changed[at] ^= 1;
            assert!(!valid_table(&changed));
        }
        let mut extra = table(false);
        extra[256] = 0;
        assert!(!valid_table(&extra));
    }
    #[test]
    fn image_extents_fail_closed() {
        let mut image = [0u8; 64];
        image[0] = 0xe9;
        image[1] = 1;
        image[12] = 9;
        image[28..32].copy_from_slice(&16u32.to_le_bytes());
        let read = |address: u32, out: &mut [u8]| {
            let offset = (address - FACTORY_START) as usize;
            out.copy_from_slice(image.get(offset..offset + out.len()).ok_or(())?);
            Ok(())
        };
        assert_eq!(application_end(read), Ok(FACTORY_START + 64));
        assert_eq!(application_end(|_, _| Err(())), Err(Error::Read));
        image[28..32].copy_from_slice(&MAX_APPLICATION_BYTES.to_le_bytes());
        assert_eq!(
            application_end(|address, out| {
                let offset = (address - FACTORY_START) as usize;
                out.copy_from_slice(&image[offset..offset + out.len()]);
                Ok(())
            }),
            Err(Error::Image)
        );
    }
    #[test]
    fn authority_excludes_factory_code_nvs_and_filesystems() {
        assert!(journal_range(PRIVATE_START, PRIVATE_BYTES));
        assert!(journal_range(PUBLIC_START, PUBLIC_BYTES));
        for (start, bytes) in [
            (0x8000, 4096),
            (0x9000, 4096),
            (0x3b000, 4096),
            (0x110000, 4096),
            (FACTORY_START, MAX_APPLICATION_BYTES),
            (0x800000, 4096),
            (PUBLIC_START + PUBLIC_BYTES, 4096),
            (PRIVATE_START - 1, 2),
            (PUBLIC_START + PUBLIC_BYTES - 1, 2),
            (u32::MAX, 2),
            (PUBLIC_START, 0),
        ] {
            assert!(!journal_range(start, bytes));
        }
    }
}
