mod common;

use common::{vectors, MetadataIx};
use crack_coin::tx::shortvec;
use crack_coin::*;

const TITLE: TokenInfo<'static> = TokenInfo {
    name: "CrackCoin",
    symbol: "CRACK",
    uri: "https://raw.githubusercontent.com/cycle-five/crack-coin/master/assets/crackcoin.json",
};

fn unhex(s: &str) -> Vec<u8> {
    assert!(s.len().is_multiple_of(2));
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

fn kp(seed_hex: &str) -> Keypair {
    Keypair::from_seed(unhex(seed_hex).try_into().unwrap())
}

fn pk(s: &str) -> Pubkey {
    s.parse().unwrap()
}

/// The message is everything after the signature block.
fn message_of(wire: &[u8], n_sigs: usize) -> &[u8] {
    assert_eq!(wire[0] as usize, n_sigs);
    &wire[1 + 64 * n_sigs..]
}

fn assert_ix_matches(ix: &Instruction, fx: &MetadataIx) {
    assert_eq!(ix.program_id, pk(&fx.program));
    let got: Vec<_> = ix
        .accounts
        .iter()
        .map(|m| (m.pubkey.to_string(), m.is_signer, m.is_writable))
        .collect();
    let want: Vec<_> = fx
        .accounts
        .iter()
        .map(|m| (m.pubkey.clone(), m.is_signer, m.is_writable))
        .collect();
    assert_eq!(got, want);
    assert_eq!(ix.data, unhex(&fx.data_hex));
}

#[test]
fn shortvec_encodes_lengths() {
    assert_eq!(shortvec(0), [0]);
    assert_eq!(shortvec(127), [0x7f]);
    assert_eq!(shortvec(128), [0x80, 0x01]);
    assert_eq!(shortvec(16383), [0xff, 0x7f]);
}

#[test]
fn mint_tx_matches_web3() {
    let v = vectors();
    let authority = kp(&v.keys.authority.seed_hex);
    let owner = pk(&v.keys.owner.pubkey);
    let mint = pk(&v.keys.mint.pubkey);
    let ata = associated_token_address(&owner, &mint);
    let ixs = [
        create_ata_idempotent(&authority.pubkey(), &owner, &mint),
        mint_to_checked(&mint, &ata, &authority.pubkey(), v.mint_tx.amount, 0),
    ];
    let tx = sign_transaction(&[&authority], &ixs, &v.blockhash.parse().unwrap()).unwrap();
    assert_eq!(message_of(&tx.wire, 1), unhex(&v.mint_tx.message_hex));
    assert_eq!(tx.signature.to_string(), v.mint_tx.signature);
    assert_eq!(tx.wire, unhex(&v.mint_tx.wire_hex));
}

#[test]
fn create_mint_tx_matches_web3() {
    let v = vectors();
    let authority = kp(&v.keys.authority.seed_hex);
    let mint = kp(&v.keys.mint.seed_hex);
    let ixs = [
        create_account(
            &authority.pubkey(),
            &mint.pubkey(),
            v.create_mint_tx.lamports,
            82,
            &TOKEN_PROGRAM,
        ),
        initialize_mint2(&mint.pubkey(), 0, &authority.pubkey(), None),
    ];
    let tx = sign_transaction(&[&authority, &mint], &ixs, &v.blockhash.parse().unwrap()).unwrap();
    assert_eq!(
        message_of(&tx.wire, 2),
        unhex(&v.create_mint_tx.message_hex)
    );
    assert_eq!(tx.signature.to_string(), v.create_mint_tx.signatures[0]);
    let sigs: Vec<String> = (0..2)
        .map(|i| Signature(tx.wire[1 + 64 * i..65 + 64 * i].try_into().unwrap()).to_string())
        .collect();
    assert_eq!(sigs, v.create_mint_tx.signatures);
    assert_eq!(tx.wire, unhex(&v.create_mint_tx.wire_hex));
}

#[test]
fn metadata_create_ix_matches_mpl() {
    let v = vectors();
    let ix = create_metadata_v3(
        &pk(&v.keys.mint.pubkey),
        &pk(&v.keys.authority.pubkey),
        &TITLE,
    );
    assert_ix_matches(&ix, &v.metadata_create_ix);
}

#[test]
fn metadata_update_ix_matches_mpl() {
    let v = vectors();
    let ix = update_metadata_v2(
        &pk(&v.keys.mint.pubkey),
        &pk(&v.keys.authority.pubkey),
        &TITLE,
    );
    assert_ix_matches(&ix, &v.metadata_update_ix);
}

#[test]
fn freeze_authority_is_encoded_when_given() {
    let m = pk(&vectors().keys.mint.pubkey);
    let ix = initialize_mint2(&m, 0, &m, Some(&SYSTEM_PROGRAM));
    assert_eq!(ix.data.len(), 2 + 32 + 1 + 32);
    assert_eq!(ix.data[34], 1);
}

#[test]
fn a_missing_signer_is_refused() {
    let v = vectors();
    let authority = pk(&v.keys.authority.pubkey);
    let owner = kp(&v.keys.owner.seed_hex);
    let mint = pk(&v.keys.mint.pubkey);
    let ata = associated_token_address(&owner.pubkey(), &mint);
    let ixs = [
        create_ata_idempotent(&authority, &owner.pubkey(), &mint),
        mint_to_checked(&mint, &ata, &authority, v.mint_tx.amount, 0),
    ];
    let r = sign_transaction(&[&owner], &ixs, &v.blockhash.parse().unwrap());
    assert!(matches!(r, Err(Error::NoSigner)), "{r:?}");
}

#[test]
fn accounts_are_grouped_and_flags_or_together() {
    let seed = |n: u8| Keypair::from_seed([n; 32]);
    let (payer, ro_signer, rw_signer) = (seed(1), seed(2), seed(3));
    let (ro, rw) = (seed(4).pubkey(), seed(5).pubkey());
    let program = seed(6).pubkey();
    let meta = |pubkey, is_signer, is_writable| AccountMeta {
        pubkey,
        is_signer,
        is_writable,
    };
    // ro_signer first appears unsigned, then as a signer: the flags OR together.
    let ixs = [Instruction {
        program_id: program,
        accounts: vec![
            meta(ro, false, false),
            meta(ro_signer.pubkey(), false, false),
            meta(ro_signer.pubkey(), true, false),
            meta(rw, false, true),
            meta(rw_signer.pubkey(), true, true),
            meta(ro, false, true), // duplicate: readonly -> writable
        ],
        data: vec![],
    }];
    let hash = Hash([9; 32]);
    let tx = sign_transaction(&[&payer, &ro_signer, &rw_signer], &ixs, &hash).unwrap();
    let msg = message_of(&tx.wire, 3);
    assert_eq!(msg[..4], [3, 1, 1, 6]); // signers, readonly signed, readonly unsigned, key count
    let keys: Vec<Pubkey> = (0..6)
        .map(|i| Pubkey(msg[4 + 32 * i..36 + 32 * i].try_into().unwrap()))
        .collect();
    let want = [
        payer.pubkey(),
        rw_signer.pubkey(),
        ro_signer.pubkey(),
        ro,
        rw,
        program,
    ];
    // Writable non-signers keep insertion order: `ro` (made writable) came before `rw`.
    assert_eq!(keys, want);
    // Each signature verifies against its slot's message, in that order.
    assert_eq!(tx.wire[1..65], tx.signature.0);
    assert_eq!(&tx.wire[65..129], &rw_signer.sign(msg).0);
    assert_eq!(&tx.wire[129..193], &ro_signer.sign(msg).0);
}
