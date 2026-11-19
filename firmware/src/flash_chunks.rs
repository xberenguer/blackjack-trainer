//! Cache-off staging: source/destination PSRAM is accessed only outside each flash-driver call.
pub const CHUNK: usize = 512;
#[repr(align(4))]
pub struct Scratch(pub [u8; CHUNK]);
/// Program word aligned physical bytes; pad only the final word with erased bits.
/// CRC and record length always cover the original, unpadded logical bytes.
pub fn program<E>(
    offset: usize,
    bytes: &[u8],
    scratch: &mut Scratch,
    mut physical: impl FnMut(usize, &[u8]) -> Result<(), E>,
) -> Result<(), E> {
    assert_eq!(offset % 4, 0);
    for (at, chunk) in bytes.chunks(CHUNK).enumerate() {
        let n = chunk.len();
        let padded = n.div_ceil(4) * 4;
        scratch.0[..padded].fill(0xff);
        scratch.0[..n].copy_from_slice(chunk);
        physical(offset + at * CHUNK, &scratch.0[..padded])?;
    }
    Ok(())
}
pub fn read<E>(
    offset: usize,
    out: &mut [u8],
    scratch: &mut Scratch,
    mut physical: impl FnMut(usize, &mut [u8]) -> Result<(), E>,
) -> Result<(), E> {
    assert_eq!(offset % 4, 0);
    for (at, chunk) in out.chunks_mut(CHUNK).enumerate() {
        let n = chunk.len();
        let padded = n.div_ceil(4) * 4;
        physical(offset + at * CHUNK, &mut scratch.0[..padded])?;
        chunk.copy_from_slice(&scratch.0[..n]);
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_tlv_lengths_are_word_aligned_and_padding_is_erased() {
        for n in 0..1600 {
            let input: Vec<u8> = (0..n).map(|v| v as u8).collect();
            let mut flash = vec![0xff; 2048];
            let mut scratch = Scratch([0; CHUNK]);
            program(4, &input, &mut scratch, |p, b| {
                assert_eq!(p % 4, 0);
                assert_eq!(b.len() % 4, 0);
                assert!(b.len() <= 512);
                flash[p..p + b.len()].copy_from_slice(b);
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(&flash[4..4 + n], &input);
            assert!(flash[4 + n..].iter().all(|&v| v == 0xff));
            let mut out = vec![0; n];
            read(4, &mut out, &mut scratch, |p, b| {
                assert_eq!(p % 4, 0);
                assert_eq!(b.len() % 4, 0);
                b.copy_from_slice(&flash[p..p + b.len()]);
                Ok::<(), ()>(())
            })
            .unwrap();
            assert_eq!(out, input);
        }
    }
}
