//! Program ids and derived addresses.

use sha2::{Digest, Sha256};

use crate::Pubkey;

pub const SYSTEM_PROGRAM: Pubkey = Pubkey([0; 32]);
pub const TOKEN_PROGRAM: Pubkey = Pubkey([
    6, 221, 246, 225, 215, 101, 161, 147, 217, 203, 225, 70, 206, 235, 121, 172, 28, 180, 133, 237,
    95, 91, 55, 145, 58, 140, 245, 133, 126, 255, 0, 169,
]);
pub const ASSOCIATED_TOKEN_PROGRAM: Pubkey = Pubkey([
    140, 151, 37, 143, 78, 36, 137, 241, 187, 61, 16, 41, 20, 142, 13, 131, 11, 90, 19, 153, 218,
    255, 16, 132, 4, 142, 123, 216, 219, 233, 248, 89,
]);
pub const TOKEN_METADATA_PROGRAM: Pubkey = Pubkey([
    11, 112, 101, 177, 227, 209, 124, 69, 56, 157, 82, 127, 107, 4, 195, 205, 88, 184, 108, 115,
    26, 160, 253, 181, 73, 182, 209, 188, 3, 248, 41, 70,
]);

/// The first off-curve `sha256(seeds ‖ bump ‖ program ‖ "ProgramDerivedAddress")`, bump 255 down.
pub fn find_program_address(seeds: &[&[u8]], program: &Pubkey) -> (Pubkey, u8) {
    for bump in (0..=255u8).rev() {
        let mut hasher = Sha256::new();
        for seed in seeds {
            hasher.update(seed);
        }
        hasher.update([bump]);
        hasher.update(program.0);
        hasher.update(b"ProgramDerivedAddress");
        let candidate = Pubkey(hasher.finalize().into());
        if !candidate.is_on_curve() {
            return (candidate, bump);
        }
    }
    unreachable!("a 256-bump search always finds an off-curve address in practice")
}

pub fn associated_token_address(owner: &Pubkey, mint: &Pubkey) -> Pubkey {
    find_program_address(
        &[&owner.0, &TOKEN_PROGRAM.0, &mint.0],
        &ASSOCIATED_TOKEN_PROGRAM,
    )
    .0
}

pub fn metadata_address(mint: &Pubkey) -> Pubkey {
    find_program_address(
        &[b"metadata", &TOKEN_METADATA_PROGRAM.0, &mint.0],
        &TOKEN_METADATA_PROGRAM,
    )
    .0
}
