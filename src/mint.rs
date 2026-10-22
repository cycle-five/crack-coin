//! The SPL Token mint account, parsed from its raw 82 bytes.

use crate::{Error, Pubkey};

/// The size of a classic SPL Token mint account.
pub const MINT_ACCOUNT_LEN: u64 = 82;

/// A decoded mint account.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MintAccount {
    pub mint_authority: Option<Pubkey>,
    pub supply: u64,
    pub decimals: u8,
    pub is_initialized: bool,
    pub freeze_authority: Option<Pubkey>,
}

fn coption(data: &[u8]) -> Result<Option<Pubkey>, Error> {
    let tag = u32::from_le_bytes(data[..4].try_into().map_err(|_| decode())?);
    let key: [u8; 32] = data[4..36].try_into().map_err(|_| decode())?;
    match tag {
        0 => Ok(None),
        1 => Ok(Some(Pubkey(key))),
        _ => Err(Error::Decode("bad option tag in mint account".into())),
    }
}

fn decode() -> Error {
    Error::Decode("malformed mint account".into())
}

impl MintAccount {
    /// Parses the first 82 bytes; anything after them (Token-2022 extensions) is ignored.
    pub fn parse(data: &[u8]) -> Result<MintAccount, Error> {
        if (data.len() as u64) < MINT_ACCOUNT_LEN {
            return Err(Error::Decode(format!(
                "mint account is {} bytes, need {MINT_ACCOUNT_LEN}",
                data.len()
            )));
        }
        let supply = u64::from_le_bytes(data[36..44].try_into().map_err(|_| decode())?);
        let is_initialized = match data[45] {
            0 => false,
            1 => true,
            _ => return Err(Error::Decode("bad initialized flag in mint account".into())),
        };
        Ok(MintAccount {
            mint_authority: coption(&data[0..36])?,
            supply,
            decimals: data[44],
            is_initialized,
            freeze_authority: coption(&data[46..82])?,
        })
    }
}
