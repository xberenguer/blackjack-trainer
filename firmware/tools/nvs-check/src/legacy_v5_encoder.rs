//! Verbatim fixed-size record writer from the sealed0.5 release, retained only as a test oracle.
const PACKET: usize = 336;
const JOURNAL_BYTES: usize = 360;
use crate::storage_format::crc32;
pub fn original_record(packet: &[u8; PACKET], sequence: u32) -> [u8; JOURNAL_BYTES] {
    let mut b = [255; JOURNAL_BYTES];
    b[..4].copy_from_slice(b"SBRJ");
    b[4..8].copy_from_slice(&1u32.to_le_bytes());
    b[8..12].copy_from_slice(&sequence.to_le_bytes());
    b[12..16].copy_from_slice(&(PACKET as u32).to_le_bytes());
    b[16..352].copy_from_slice(packet);
    let c = crc32(&b[..352]);
    b[352..356].copy_from_slice(&c.to_le_bytes());
    b[356..360].copy_from_slice(b"DONE");
    b
}
