mod common;

use common::vectors;
use crack_coin::{
    Error, Keypair, Pubkey, ASSOCIATED_TOKEN_PROGRAM, SYSTEM_PROGRAM, TOKEN_METADATA_PROGRAM,
    TOKEN_PROGRAM,
};

fn seed(hex: &str) -> [u8; 32] {
    let mut out = [0u8; 32];
    for (i, b) in out.iter_mut().enumerate() {
        *b = u8::from_str_radix(&hex[2 * i..2 * i + 2], 16).unwrap();
    }
    out
}

fn cli_json(bytes: &[u8]) -> String {
    let parts: Vec<String> = bytes.iter().map(|b| b.to_string()).collect();
    format!("[{}]", parts.join(","))
}

fn authority_bytes() -> Vec<u8> {
    let v = vectors();
    let s = seed(&v.keys.authority.seed_hex);
    let pk: Pubkey = v.keys.authority.pubkey.parse().unwrap();
    [s.as_slice(), pk.0.as_slice()].concat()
}

#[test]
fn program_ids_display_as_their_base58() {
    let v = vectors();
    assert_eq!(SYSTEM_PROGRAM.to_string(), v.programs.system);
    assert_eq!(TOKEN_PROGRAM.to_string(), v.programs.token);
    assert_eq!(
        ASSOCIATED_TOKEN_PROGRAM.to_string(),
        v.programs.associated_token
    );
    assert_eq!(TOKEN_METADATA_PROGRAM.to_string(), v.programs.metadata);
}

#[test]
fn pubkey_round_trips_base58() {
    let v = vectors();
    let pk: Pubkey = v.keys.owner.pubkey.parse().unwrap();
    assert_eq!(pk.to_string(), v.keys.owner.pubkey);
    assert_eq!(format!("{pk:?}"), v.keys.owner.pubkey);
}

#[test]
fn pubkey_refuses_bad_base58_and_wrong_length() {
    assert!(matches!("0OIl".parse::<Pubkey>(), Err(Error::Base58)));
    let short = bs58::encode([1u8; 31]).into_string();
    assert!(matches!(
        short.parse::<Pubkey>(),
        Err(Error::Length {
            expected: 32,
            got: 31
        })
    ));
}

#[test]
fn keypair_reads_the_cli_format() {
    let v = vectors();
    let kp = Keypair::from_json(&cli_json(&authority_bytes())).unwrap();
    assert_eq!(kp.pubkey().to_string(), v.keys.authority.pubkey);
}

#[test]
fn keypair_trims_a_pasted_newline() {
    let text = format!("  {}\n", cli_json(&authority_bytes()));
    assert!(Keypair::from_json(&text).is_ok());
}

#[test]
fn keypair_refuses_a_mismatched_public_half() {
    let mut bytes = authority_bytes();
    *bytes.last_mut().unwrap() ^= 1;
    assert!(matches!(
        Keypair::from_json(&cli_json(&bytes)),
        Err(Error::BadKeypair)
    ));
}

#[test]
fn keypair_refuses_wrong_lengths_and_garbage() {
    assert!(matches!(
        Keypair::from_json("[1,2,3]"),
        Err(Error::BadKeypair)
    ));
    assert!(matches!(
        Keypair::from_json("not json"),
        Err(Error::BadKeypair)
    ));
}

#[test]
fn keypair_debug_shows_only_the_public_key() {
    let kp = Keypair::from_seed(seed(&vectors().keys.owner.seed_hex));
    assert_eq!(format!("{kp:?}"), format!("Keypair({})", kp.pubkey()));
}

#[test]
fn keypair_errors_never_echo_the_input() {
    let err = Keypair::from_json("[7777, \"7777\"]").unwrap_err();
    assert!(!err.to_string().contains("7777"));
    let err = Keypair::from_json("7777").unwrap_err();
    assert!(!err.to_string().contains("7777"));
}

#[test]
fn secret_json_round_trips() {
    let kp = Keypair::from_seed([9u8; 32]);
    let back = Keypair::from_json(&kp.secret_json()).unwrap();
    assert_eq!(back.pubkey(), kp.pubkey());
}

#[test]
fn signatures_verify_and_round_trip_base58() {
    use ed25519_dalek::{Signature as Sig, Verifier, VerifyingKey};
    let kp = Keypair::from_seed([5u8; 32]);
    let sig = kp.sign(b"hello");
    let vk = VerifyingKey::from_bytes(&kp.pubkey().0).unwrap();
    vk.verify(b"hello", &Sig::from_bytes(&sig.0)).unwrap();
    assert_eq!(
        sig.to_string().parse::<crack_coin::Signature>().unwrap(),
        sig
    );
}
