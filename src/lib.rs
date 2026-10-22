//! CrackCoin: a worthless-by-design Solana devnet token for the CrackTunes Discord bot.
//!
//! The crate speaks raw JSON-RPC to Solana and hand-encodes its own transactions; it has no
//! dependency on the `solana-*` crates. It refuses mainnet: [`Devnet::connect`] is the only way to
//! get a client and it returns [`Error::Mainnet`] if the endpoint reports mainnet-beta's genesis
//! hash.
//!
//! Main entry points:
//! - [`Coin::open`] connects, verifies the mint and its authority, and then
//!   [`Coin::prepare_mint`] signs a mint to a member, [`Devnet::send`] submits it, and
//!   [`wait_for`] follows it to a [`TxStatus`]. [`Coin::has_token_account`] checks a member's
//!   token account.
//! - [`create_mint`] and [`set_metadata`] set the token up; [`check_mint`] verifies it.
//! - [`Keypair`], [`Pubkey`], [`Signature`] and [`Hash`] are the key types.
//!
//! The `cli` feature builds the `crack-coin` binary (keygen, airdrop, create-mint, ...).

pub mod address;
pub mod cluster;
pub mod coin;
pub mod ix;
pub mod key;
pub mod mint;
pub mod rpc;
pub mod tx;

pub use address::{
    associated_token_address, find_program_address, metadata_address, ASSOCIATED_TOKEN_PROGRAM,
    SYSTEM_PROGRAM, TOKEN_METADATA_PROGRAM, TOKEN_PROGRAM,
};
pub use cluster::{refuse_mainnet, DEFAULT_RPC_URL, DEVNET_GENESIS, MAINNET_GENESIS};
#[cfg(unix)]
pub use coin::write_keypair_file;
pub use coin::{
    check_mint, create_mint, mint_transaction, read_metadata, set_metadata, wait_for, Coin,
    PreparedMint, TOKEN_ACCOUNT_LEN,
};
pub use ix::{
    create_account, create_ata_idempotent, create_metadata_v3, initialize_mint2, mint_to_checked,
    update_metadata_v2, TokenInfo,
};
pub use key::{Hash, Keypair, Pubkey, Signature};
pub use mint::{MintAccount, MINT_ACCOUNT_LEN};
pub use rpc::{Devnet, TxStatus, RPC_TIMEOUT};

pub use tx::{sign_transaction, AccountMeta, Instruction, SignedTx};

/// The token's name.
pub const NAME: &str = "CrackCoin";
/// The token's symbol.
pub const SYMBOL: &str = "CRACK";
/// Where the token's off-chain metadata JSON lives.
pub const METADATA_URI: &str =
    "https://raw.githubusercontent.com/cycle-five/crack-coin/master/assets/crackcoin.json";

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
