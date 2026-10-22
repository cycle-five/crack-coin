//! CrackCoin: a worthless-by-design Solana devnet token for the CrackTunes Discord bot.
//!
//! This crate never talks to mainnet. The modules arrive task by task.

pub mod address;
pub mod ix;
pub mod key;
pub mod tx;

pub use address::{
    associated_token_address, find_program_address, metadata_address, ASSOCIATED_TOKEN_PROGRAM,
    SYSTEM_PROGRAM, TOKEN_METADATA_PROGRAM, TOKEN_PROGRAM,
};
pub use ix::{
    create_account, create_ata_idempotent, create_metadata_v3, initialize_mint2, mint_to_checked,
    update_metadata_v2, TokenInfo,
};
pub use key::{Hash, Keypair, Pubkey, Signature};

pub use tx::{sign_transaction, AccountMeta, Instruction, SignedTx};

/// Every failure this crate reports. Messages never include caller-supplied text, so a pasted
/// secret cannot leak through an error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("not valid base58")]
    Base58,
    #[error("wrong length: expected {expected} bytes, got {got}")]
    Length { expected: usize, got: usize },
    #[error("not a valid keypair")]
    BadKeypair,
    #[error("no signer for a required signature")]
    NoSigner,
    #[error("rpc error {code}: {message}")]
    Rpc { code: i64, message: String },
    #[error("http error: {0}")]
    Http(String),
    #[error("could not decode: {0}")]
    Decode(String),
    #[error("refusing to touch mainnet")]
    Mainnet,
    #[error("mint mismatch: {0}")]
    MintMismatch(&'static str),
    #[error("io error: {0}")]
    Io(String),
}
