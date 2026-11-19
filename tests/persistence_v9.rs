use stardust_blackjack::persistence::{self, Save};
#[test]
fn public_v9_default_off_optin_and_every_old_schema_migrate() {
    assert!(!Save::default().resume_enabled);
    for b in [
        include_bytes!("data/save-v1.bin").as_slice(),
        include_bytes!("data/save-v2.bin").as_slice(),
        include_bytes!("data/save-v3.bin").as_slice(),
        include_bytes!("data/save-v4.bin").as_slice(),
        include_bytes!("data/save-v5.bin").as_slice(),
        include_bytes!("data/save-v6.bin").as_slice(),
        include_bytes!("data/save-v7.bin").as_slice(),
        include_bytes!("data/save-v8.bin").as_slice(),
    ] {
        let old = Save::decode(b).unwrap();
        assert!(!old.resume_enabled);
        let mut enabled = old;
        enabled.resume_enabled = true;
        let packet = enabled.encode().unwrap();
        assert_eq!(packet[583], 1);
        assert_eq!(Save::decode(&packet), Some(enabled));
    }
}
#[test]
fn old_reserved_bit_and_new_invalid_bool_are_rejected() {
    let old = include_bytes!("data/save-v8.bin");
    let mut packet = *old;
    packet[583] = 1;
    let end = packet.len() - 4;
    let crc = persistence::crc32(&packet[..end]);
    packet[end..].copy_from_slice(&crc.to_le_bytes());
    assert!(Save::decode(&packet).is_none());
    let mut packet = Save::default().encode().unwrap();
    packet[583] = 2;
    let end = packet.len() - 4;
    let crc = persistence::crc32(&packet[..end]);
    packet[end..].copy_from_slice(&crc.to_le_bytes());
    assert!(Save::decode(&packet).is_none());
}
