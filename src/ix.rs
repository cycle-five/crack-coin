//! The six instructions this crate sends.

use crate::tx::{AccountMeta, Instruction};
use crate::{
    associated_token_address, metadata_address, Pubkey, ASSOCIATED_TOKEN_PROGRAM, SYSTEM_PROGRAM,
    TOKEN_METADATA_PROGRAM, TOKEN_PROGRAM,
};

/// The on-chain name, symbol and metadata uri of a token.
#[derive(Debug, Clone, Copy)]
pub struct TokenInfo<'a> {
    pub name: &'a str,
    pub symbol: &'a str,
    pub uri: &'a str,
}

fn borsh_str(out: &mut Vec<u8>, s: &str) {
    out.extend_from_slice(&(s.len() as u32).to_le_bytes());
    out.extend_from_slice(s.as_bytes());
}

/// Borsh `DataV2`: strings, fee 0, and no creators, collection or uses.
fn data_v2(out: &mut Vec<u8>, info: &TokenInfo<'_>) {
    borsh_str(out, info.name);
    borsh_str(out, info.symbol);
    borsh_str(out, info.uri);
    out.extend_from_slice(&[0, 0]); // seller_fee_basis_points
    out.extend_from_slice(&[0, 0, 0]); // creators, collection, uses = None
}

pub fn create_ata_idempotent(payer: &Pubkey, owner: &Pubkey, mint: &Pubkey) -> Instruction {
    Instruction {
        program_id: ASSOCIATED_TOKEN_PROGRAM,
        accounts: vec![
            AccountMeta::new(*payer, true),
            AccountMeta::new(associated_token_address(owner, mint), false),
            AccountMeta::readonly(*owner, false),
            AccountMeta::readonly(*mint, false),
            AccountMeta::readonly(SYSTEM_PROGRAM, false),
            AccountMeta::readonly(TOKEN_PROGRAM, false),
        ],
        data: vec![1],
    }
}

pub fn mint_to_checked(
    mint: &Pubkey,
    dest: &Pubkey,
    authority: &Pubkey,
    amount: u64,
    decimals: u8,
) -> Instruction {
    let mut data = vec![14];
    data.extend_from_slice(&amount.to_le_bytes());
    data.push(decimals);
    Instruction {
        program_id: TOKEN_PROGRAM,
        accounts: vec![
            AccountMeta::new(*mint, false),
            AccountMeta::new(*dest, false),
            AccountMeta::readonly(*authority, true),
        ],
        data,
    }
}

pub fn create_account(
    from: &Pubkey,
    new: &Pubkey,
    lamports: u64,
    space: u64,
    owner: &Pubkey,
) -> Instruction {
    let mut data = 0u32.to_le_bytes().to_vec();
    data.extend_from_slice(&lamports.to_le_bytes());
    data.extend_from_slice(&space.to_le_bytes());
    data.extend_from_slice(&owner.0);
    Instruction {
        program_id: SYSTEM_PROGRAM,
        accounts: vec![AccountMeta::new(*from, true), AccountMeta::new(*new, true)],
        data,
    }
}

pub fn initialize_mint2(
    mint: &Pubkey,
    decimals: u8,
    mint_authority: &Pubkey,
    freeze: Option<&Pubkey>,
) -> Instruction {
    let mut data = vec![20, decimals];
    data.extend_from_slice(&mint_authority.0);
    match freeze {
        None => data.push(0),
        Some(f) => {
            data.push(1);
            data.extend_from_slice(&f.0);
        },
    }
    Instruction {
        program_id: TOKEN_PROGRAM,
        accounts: vec![AccountMeta::new(*mint, false)],
        data,
    }
}

/// Metaplex `CreateMetadataAccountV3`, mutable, no collection details. The authority is mint
/// authority, payer and update authority.
pub fn create_metadata_v3(mint: &Pubkey, authority: &Pubkey, info: &TokenInfo<'_>) -> Instruction {
    let mut data = vec![33];
    data_v2(&mut data, info);
    data.push(1); // is_mutable
    data.push(0); // collection_details = None
    Instruction {
        program_id: TOKEN_METADATA_PROGRAM,
        accounts: vec![
            AccountMeta::new(metadata_address(mint), false),
            AccountMeta::readonly(*mint, false),
            AccountMeta::readonly(*authority, true),
            AccountMeta::new(*authority, true),
            AccountMeta::readonly(*authority, true),
            AccountMeta::readonly(SYSTEM_PROGRAM, false),
        ],
        data,
    }
}

/// Metaplex `UpdateMetadataAccountV2`: new data only; authority, sale and mutability untouched.
pub fn update_metadata_v2(mint: &Pubkey, authority: &Pubkey, info: &TokenInfo<'_>) -> Instruction {
    let mut data = vec![15, 1]; // Some(DataV2)
    data_v2(&mut data, info);
    data.extend_from_slice(&[0, 0, 0]); // update_authority, primary_sale, is_mutable = None
    Instruction {
        program_id: TOKEN_METADATA_PROGRAM,
        accounts: vec![
            AccountMeta::new(metadata_address(mint), false),
            AccountMeta::readonly(*authority, true),
        ],
        data,
    }
}
