//! The CrackCoin mint: checking it, creating it, describing it and minting to members.

use std::io::Write;
use std::path::Path;
use std::time::Duration;

use crate::{
    associated_token_address, create_account, create_ata_idempotent, create_metadata_v3,
    initialize_mint2, metadata_address, mint_to_checked, sign_transaction, update_metadata_v2,
    Devnet, Error, Hash, Keypair, MintAccount, Pubkey, Signature, SignedTx, TokenInfo, TxStatus,
    MINT_ACCOUNT_LEN, TOKEN_PROGRAM,
};

/// The size of an SPL token account.
pub const TOKEN_ACCOUNT_LEN: usize = 165;

/// How often [`wait_for`] asks the cluster.
const POLL_EVERY: Duration = Duration::from_secs(2);

/// Checks that `data` is the initialized, zero-decimal mint that `authority` controls.
pub fn check_mint(data: Option<&[u8]>, authority: &Pubkey) -> Result<MintAccount, Error> {
    let data = data.ok_or(Error::MintMismatch("absent"))?;
    let mint = MintAccount::parse(data).map_err(|_| Error::MintMismatch("not a mint"))?;
    if !mint.is_initialized {
        return Err(Error::MintMismatch("not initialized"));
    }
    if mint.decimals != 0 {
        return Err(Error::MintMismatch("decimals"));
    }
    if mint.mint_authority.as_ref() != Some(authority) {
        return Err(Error::MintMismatch("authority"));
    }
    Ok(mint)
}

/// A signed mint transaction, ready to send, with the height after which it can no longer land.
#[derive(Debug, Clone)]
pub struct PreparedMint {
    pub signature: Signature,
    pub wire: Vec<u8>,
    pub last_valid_block_height: u64,
}

/// The authority creates the owner's token account if needed and mints `amount` into it.
pub fn mint_transaction(
    authority: &Keypair,
    mint: &Pubkey,
    owner: &Pubkey,
    amount: u64,
    blockhash: &Hash,
) -> Result<SignedTx, Error> {
    let payer = authority.pubkey();
    let ata = associated_token_address(owner, mint);
    let ixs = [
        create_ata_idempotent(&payer, owner, mint),
        mint_to_checked(mint, &ata, &payer, amount, 0),
    ];
    sign_transaction(&[authority], &ixs, blockhash)
}

/// An open, verified CrackCoin mint on devnet.
pub struct Coin {
    devnet: Devnet,
    authority: Keypair,
    mint: Pubkey,
    genesis: Hash,
}

impl Coin {
    /// Connects (refusing mainnet) and checks that `mint` is our mint, controlled by `authority`.
    pub async fn open(url: &str, authority: Keypair, mint: Pubkey) -> Result<Coin, Error> {
        let (devnet, genesis) = Devnet::connect(url).await?;
        let data = devnet.account_data(&mint).await?;
        check_mint(data.as_deref(), &authority.pubkey())?;
        Ok(Coin {
            devnet,
            authority,
            mint,
            genesis,
        })
    }

    pub fn mint(&self) -> &Pubkey {
        &self.mint
    }

    pub fn authority(&self) -> Pubkey {
        self.authority.pubkey()
    }

    pub fn genesis(&self) -> &Hash {
        &self.genesis
    }

    pub fn devnet(&self) -> &Devnet {
        &self.devnet
    }

    /// Builds and signs, but does not send, a mint of `amount` to `owner`.
    pub async fn prepare_mint(&self, owner: &Pubkey, amount: u64) -> Result<PreparedMint, Error> {
        let (blockhash, last_valid_block_height) = self.devnet.latest_blockhash().await?;
        let tx = mint_transaction(&self.authority, &self.mint, owner, amount, &blockhash)?;
        Ok(PreparedMint {
            signature: tx.signature,
            wire: tx.wire,
            last_valid_block_height,
        })
    }

    /// Whether `owner` already has a token account for this mint.
    pub async fn has_token_account(&self, owner: &Pubkey) -> Result<bool, Error> {
        let ata = associated_token_address(owner, &self.mint);
        let data = self.devnet.account_data(&ata).await?;
        Ok(is_token_account(data.as_deref()))
    }
}

/// Whether `data` is a token account: the account exists and is at least [`TOKEN_ACCOUNT_LEN`]
/// bytes. A lamport-only account at the address has empty data and does not count.
fn is_token_account(data: Option<&[u8]>) -> bool {
    data.is_some_and(|d| d.len() >= TOKEN_ACCOUNT_LEN)
}

/// Polls every two seconds until the transaction landed or failed, or until it is provably gone:
/// not found while the finalized height has passed `last_valid_block_height`.
pub async fn wait_for(
    devnet: &Devnet,
    sig: &Signature,
    last_valid_block_height: u64,
) -> Result<TxStatus, Error> {
    loop {
        // Height first, then status (see `Devnet::finalized_block_height`).
        let height = devnet.finalized_block_height().await?;
        match devnet.status(sig).await? {
            done @ (TxStatus::Landed | TxStatus::Failed(_)) => return Ok(done),
            TxStatus::NotFound => {
                if height > last_valid_block_height {
                    return Ok(TxStatus::NotFound);
                }
            },
            TxStatus::Processed => {},
        }
        tokio::time::sleep(POLL_EVERY).await;
    }
}

/// Sends `wire` and waits for it; anything but `Landed` is an error.
async fn send_and_land(
    devnet: &Devnet,
    tx: SignedTx,
    last_valid_block_height: u64,
) -> Result<Signature, Error> {
    devnet.send(&tx.wire).await?;
    match wait_for(devnet, &tx.signature, last_valid_block_height).await? {
        TxStatus::Landed => Ok(tx.signature),
        TxStatus::Failed(err) => Err(Error::Rpc {
            code: -2,
            message: format!("transaction failed: {err}"),
        }),
        TxStatus::NotFound | TxStatus::Processed => Err(Error::Rpc {
            code: -3,
            message: "transaction expired before it landed".into(),
        }),
    }
}

/// Creates `mint` as a zero-decimal mint controlled by `authority`, with no freeze authority.
pub async fn create_mint(
    devnet: &Devnet,
    authority: &Keypair,
    mint: &Keypair,
) -> Result<Signature, Error> {
    let lamports = devnet.rent_exempt_minimum(MINT_ACCOUNT_LEN).await?;
    let (blockhash, last_valid) = devnet.latest_blockhash().await?;
    let payer = authority.pubkey();
    let ixs = [
        create_account(
            &payer,
            &mint.pubkey(),
            lamports,
            MINT_ACCOUNT_LEN,
            &TOKEN_PROGRAM,
        ),
        initialize_mint2(&mint.pubkey(), 0, &payer, None),
    ];
    let tx = sign_transaction(&[authority, mint], &ixs, &blockhash)?;
    send_and_land(devnet, tx, last_valid).await
}

/// Creates the token's metadata when there is none, otherwise updates it.
pub async fn set_metadata(
    devnet: &Devnet,
    authority: &Keypair,
    mint: &Pubkey,
    info: &TokenInfo<'_>,
) -> Result<Signature, Error> {
    let payer = authority.pubkey();
    let ix = match devnet.account_data(&metadata_address(mint)).await? {
        None => create_metadata_v3(mint, &payer, info),
        Some(_) => update_metadata_v2(mint, &payer, info),
    };
    let (blockhash, last_valid) = devnet.latest_blockhash().await?;
    let tx = sign_transaction(&[authority], &[ix], &blockhash)?;
    send_and_land(devnet, tx, last_valid).await
}

/// Writes the keypair as Solana CLI JSON to a new file with mode 0600. Never overwrites.
/// Unix only.
#[cfg(unix)]
pub fn write_keypair_file(path: &Path, kp: &Keypair) -> Result<(), Error> {
    use std::os::unix::fs::OpenOptionsExt;
    let io = |e: std::io::Error| Error::Io(e.kind().to_string());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(path)
        .map_err(io)?;
    file.write_all(kp.secret_json().as_bytes()).map_err(io)?;
    file.sync_all().map_err(io)
}

fn take<'a>(data: &'a [u8], at: &mut usize, n: usize) -> Result<&'a [u8], Error> {
    let end = at.checked_add(n).filter(|e| *e <= data.len());
    let end = end.ok_or_else(|| Error::Decode("metadata account is truncated".into()))?;
    let out = &data[*at..end];
    *at = end;
    Ok(out)
}

fn borsh_string(data: &[u8], at: &mut usize) -> Result<String, Error> {
    let len = u32::from_le_bytes(
        take(data, at, 4)?
            .try_into()
            .map_err(|_| Error::Decode("metadata account is truncated".into()))?,
    );
    let raw = take(data, at, len as usize)?;
    let text =
        std::str::from_utf8(raw).map_err(|_| Error::Decode("metadata text is not utf-8".into()))?;
    // Metaplex stores name, symbol and uri fixed-width, padded with NUL bytes.
    Ok(text.trim_end_matches('\0').to_owned())
}

/// Best-effort decode of a Metaplex metadata account: `(name, symbol, uri)`.
pub fn read_metadata(data: &[u8]) -> Result<(String, String, String), Error> {
    let mut at = 1 + 32 + 32; // key, update authority, mint
    let name = borsh_string(data, &mut at)?;
    let symbol = borsh_string(data, &mut at)?;
    let uri = borsh_string(data, &mut at)?;
    Ok((name, symbol, uri))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_full_size_account_is_a_token_account() {
        assert!(!is_token_account(None));
        assert!(!is_token_account(Some(&[])));
        assert!(!is_token_account(Some(&[0u8; TOKEN_ACCOUNT_LEN - 1])));
        assert!(is_token_account(Some(&[0u8; TOKEN_ACCOUNT_LEN])));
    }
}
