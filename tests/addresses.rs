mod common;

use common::vectors;
use crack_coin::{
    associated_token_address, find_program_address, metadata_address, Pubkey,
    ASSOCIATED_TOKEN_PROGRAM, TOKEN_METADATA_PROGRAM, TOKEN_PROGRAM,
};

#[test]
fn on_curve_matches_web3() {
    for e in vectors().on_curve {
        let pk: Pubkey = e.pubkey.parse().unwrap();
        assert_eq!(pk.is_on_curve(), e.on_curve, "{}", e.pubkey);
    }
}

#[test]
fn ata_matches_spl_token() {
    let a = vectors().ata;
    let owner: Pubkey = a.owner.parse().unwrap();
    let mint: Pubkey = a.mint.parse().unwrap();
    assert_eq!(
        associated_token_address(&owner, &mint).to_string(),
        a.address
    );
    let (addr, bump) = find_program_address(
        &[&owner.0, &TOKEN_PROGRAM.0, &mint.0],
        &ASSOCIATED_TOKEN_PROGRAM,
    );
    assert_eq!(addr.to_string(), a.address);
    assert_eq!(bump, a.bump);
}

#[test]
fn metadata_address_matches_mpl() {
    let m = vectors().metadata_address;
    let mint: Pubkey = m.mint.parse().unwrap();
    assert_eq!(metadata_address(&mint).to_string(), m.address);
    let (_, bump) = find_program_address(
        &[b"metadata", &TOKEN_METADATA_PROGRAM.0, &mint.0],
        &TOKEN_METADATA_PROGRAM,
    );
    assert_eq!(bump, m.bump);
}
