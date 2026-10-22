mod common;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use common::vectors;
#[cfg(unix)]
use crack_coin::coin::write_keypair_file;
use crack_coin::coin::{check_mint, mint_transaction, read_metadata};
use crack_coin::*;
use serde::Deserialize;

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn kp(seed_hex: &str) -> Keypair {
    Keypair::from_seed(unhex(seed_hex).try_into().unwrap())
}

#[test]
fn mint_transaction_matches_the_fixture() {
    let v = vectors();
    let authority = kp(&v.keys.authority.seed_hex);
    let owner: Pubkey = v.keys.owner.pubkey.parse().unwrap();
    let mint: Pubkey = v.keys.mint.pubkey.parse().unwrap();
    let tx = mint_transaction(
        &authority,
        &mint,
        &owner,
        v.mint_tx.amount,
        &v.blockhash.parse().unwrap(),
    )
    .unwrap();
    assert_eq!(tx.signature.to_string(), v.mint_tx.signature);
    assert_eq!(tx.wire, unhex(&v.mint_tx.wire_hex));
}

fn mint_bytes(authority: &Pubkey, decimals: u8, init: u8) -> Vec<u8> {
    let mut d = Vec::new();
    d.extend_from_slice(&1u32.to_le_bytes());
    d.extend_from_slice(&authority.0);
    d.extend_from_slice(&0u64.to_le_bytes());
    d.push(decimals);
    d.push(init);
    d.extend_from_slice(&0u32.to_le_bytes());
    d.extend_from_slice(&[0u8; 32]);
    d
}

#[test]
fn check_mint_names_each_mismatch() {
    let me = Pubkey([1; 32]);
    let other = Pubkey([2; 32]);
    let why = |r: Result<MintAccount, Error>| match r {
        Err(Error::MintMismatch(w)) => w,
        other => panic!("expected a mismatch, got {other:?}"),
    };
    assert_eq!(why(check_mint(None, &me)), "absent");
    assert_eq!(why(check_mint(Some(&[0; 10]), &me)), "not a mint");
    assert_eq!(
        why(check_mint(Some(&mint_bytes(&me, 0, 0)), &me)),
        "not initialized"
    );
    assert_eq!(
        why(check_mint(Some(&mint_bytes(&me, 2, 1)), &me)),
        "decimals"
    );
    assert_eq!(
        why(check_mint(Some(&mint_bytes(&other, 0, 1)), &me)),
        "authority"
    );
    assert!(check_mint(Some(&mint_bytes(&me, 0, 1)), &me).is_ok());
}

#[cfg(unix)]
#[test]
fn keypair_file_is_0600_and_never_overwritten() {
    let path = std::env::temp_dir().join(format!(
        "crack-coin-test-{}-{}.json",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let first = Keypair::from_seed([5; 32]);
    write_keypair_file(&path, &first).unwrap();
    let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
    let back = Keypair::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(back.pubkey(), first.pubkey());
    assert!(write_keypair_file(&path, &Keypair::from_seed([6; 32])).is_err());
    let still = Keypair::from_json(&std::fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(still.pubkey(), first.pubkey());
    std::fs::remove_file(&path).unwrap();
}

#[derive(Deserialize)]
struct TokenJson {
    name: String,
    symbol: String,
    description: String,
    image: String,
}

#[test]
fn the_metadata_json_names_the_coin() {
    let raw = include_str!("../assets/crackcoin.json");
    let j: TokenJson = serde_json::from_str(raw).unwrap();
    assert_eq!(j.name, NAME);
    assert_eq!(j.symbol, SYMBOL);
    assert_eq!(
        j.image,
        "https://raw.githubusercontent.com/cycle-five/crack-coin/master/assets/crackcoin.png"
    );
    assert!(!j.description.is_empty());
}

fn padded(s: &str, width: usize) -> Vec<u8> {
    let mut b = s.as_bytes().to_vec();
    b.resize(width, 0);
    let mut out = (width as u32).to_le_bytes().to_vec();
    out.extend(b);
    out
}

#[test]
fn read_metadata_decodes_and_trims_the_nul_padding() {
    let mut d = vec![4u8];
    d.extend_from_slice(&[7; 32]); // update authority
    d.extend_from_slice(&[8; 32]); // mint
    d.extend(padded(NAME, 32));
    d.extend(padded(SYMBOL, 10));
    d.extend(padded(METADATA_URI, 200));
    d.extend_from_slice(&[0, 0, 1]); // seller fee, trailing bytes
    let (name, symbol, uri) = read_metadata(&d).unwrap();
    assert_eq!(
        (name.as_str(), symbol.as_str(), uri.as_str()),
        (NAME, SYMBOL, METADATA_URI)
    );
    assert!(matches!(read_metadata(&d[..70]), Err(Error::Decode(_))));
}

#[test]
fn the_token_constants_are_exact() {
    assert_eq!(NAME, "CrackCoin");
    assert_eq!(SYMBOL, "CRACK");
    assert_eq!(
        METADATA_URI,
        "https://raw.githubusercontent.com/cycle-five/crack-coin/master/assets/crackcoin.json"
    );
}
