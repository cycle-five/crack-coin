use crack_coin::{Error, MintAccount, Pubkey, MINT_ACCOUNT_LEN};

fn bytes() -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&1u32.to_le_bytes());
    d.extend_from_slice(&[3u8; 32]);
    d.extend_from_slice(&7u64.to_le_bytes());
    d.push(0);
    d.push(1);
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&[0u8; 32]);
    d
}

#[test]
fn mint_account_parses_82_bytes() {
    let d = bytes();
    assert_eq!(d.len() as u64, MINT_ACCOUNT_LEN);
    let m = MintAccount::parse(&d).unwrap();
    assert_eq!(m.mint_authority, Some(Pubkey([3u8; 32])));
    assert_eq!(m.supply, 7);
    assert_eq!(m.decimals, 0);
    assert!(m.is_initialized);
    assert_eq!(m.freeze_authority, None);
}

#[test]
fn mint_account_refuses_short_data() {
    let d = bytes();
    assert!(matches!(
        MintAccount::parse(&d[..81]),
        Err(Error::Decode(_))
    ));
    assert!(matches!(MintAccount::parse(&[]), Err(Error::Decode(_))));
}

#[test]
fn mint_account_ignores_extension_bytes_and_reads_freeze_authority() {
    let mut d = bytes();
    d[46..50].copy_from_slice(&1u32.to_le_bytes());
    d[50..82].copy_from_slice(&[4u8; 32]);
    d.extend_from_slice(&[9u8; 10]);
    let m = MintAccount::parse(&d).unwrap();
    assert_eq!(m.freeze_authority, Some(Pubkey([4u8; 32])));
}

#[test]
fn mint_account_rejects_a_bad_option_tag() {
    let mut d = bytes();
    d[0] = 2;
    assert!(matches!(MintAccount::parse(&d), Err(Error::Decode(_))));
}
