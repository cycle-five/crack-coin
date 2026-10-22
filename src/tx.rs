//! Legacy Solana transactions: instruction types, the message compiler and signing.

use crate::{Error, Hash, Keypair, Pubkey, Signature};

/// One account an instruction touches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountMeta {
    pub pubkey: Pubkey,
    pub is_signer: bool,
    pub is_writable: bool,
}

impl AccountMeta {
    pub fn new(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: true,
        }
    }

    pub fn readonly(pubkey: Pubkey, is_signer: bool) -> Self {
        Self {
            pubkey,
            is_signer,
            is_writable: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instruction {
    pub program_id: Pubkey,
    pub accounts: Vec<AccountMeta>,
    pub data: Vec<u8>,
}

/// A signed transaction ready for `sendTransaction`.
#[derive(Debug, Clone)]
pub struct SignedTx {
    /// The fee payer's signature, which is the transaction id.
    pub signature: Signature,
    pub wire: Vec<u8>,
}

/// Solana's compact-u16 length prefix.
pub fn shortvec(mut len: u16) -> Vec<u8> {
    let mut out = Vec::new();
    loop {
        let byte = (len & 0x7f) as u8;
        len >>= 7;
        if len == 0 {
            out.push(byte);
            return out;
        }
        out.push(byte | 0x80);
    }
}

#[derive(Clone)]
struct Key {
    pubkey: Pubkey,
    is_signer: bool,
    is_writable: bool,
}

/// Collect keys the way Solana's `CompiledKeys` does: payer first, then per instruction the
/// program id and its metas. A duplicate ORs its flags.
fn collect_keys(payer: &Pubkey, ixs: &[Instruction]) -> Vec<Key> {
    let mut keys = vec![Key {
        pubkey: *payer,
        is_signer: true,
        is_writable: true,
    }];
    let mut add = |pubkey: &Pubkey, is_signer: bool, is_writable: bool| {
        if let Some(k) = keys.iter_mut().find(|k| k.pubkey == *pubkey) {
            k.is_signer |= is_signer;
            k.is_writable |= is_writable;
        } else {
            keys.push(Key {
                pubkey: *pubkey,
                is_signer,
                is_writable,
            });
        }
    };
    for ix in ixs {
        add(&ix.program_id, false, false);
        for m in &ix.accounts {
            add(&m.pubkey, m.is_signer, m.is_writable);
        }
    }
    keys
}

/// Compiled message bytes plus the number of required signatures (the signer keys come first).
fn compile(payer: &Pubkey, ixs: &[Instruction], blockhash: &Hash) -> (Vec<u8>, Vec<Pubkey>) {
    let keys = collect_keys(payer, ixs);
    let pick = |signer: bool, writable: bool| {
        keys.iter()
            .filter(move |k| k.is_signer == signer && k.is_writable == writable)
            .map(|k| k.pubkey)
            .collect::<Vec<_>>()
    };
    let (ws, rs, wn, rn) = (
        pick(true, true),
        pick(true, false),
        pick(false, true),
        pick(false, false),
    );
    let num_signers = ws.len() + rs.len();
    let ordered: Vec<Pubkey> = [ws, rs.clone(), wn, rn.clone()].concat();

    let mut msg = vec![num_signers as u8, rs.len() as u8, rn.len() as u8];
    msg.extend(shortvec(ordered.len() as u16));
    for k in &ordered {
        msg.extend_from_slice(&k.0);
    }
    msg.extend_from_slice(&blockhash.0);
    msg.extend(shortvec(ixs.len() as u16));
    let index = |p: &Pubkey| {
        ordered
            .iter()
            .position(|k| k == p)
            .expect("collected above") as u8
    };
    for ix in ixs {
        msg.push(index(&ix.program_id));
        msg.extend(shortvec(ix.accounts.len() as u16));
        msg.extend(ix.accounts.iter().map(|m| index(&m.pubkey)));
        msg.extend(shortvec(ix.data.len() as u16));
        msg.extend_from_slice(&ix.data);
    }
    let signers = ordered[..num_signers].to_vec();
    (msg, signers)
}

/// Build and sign a legacy transaction. `signers[0]` pays the fee. A required signer with no
/// matching keypair gives [`Error::NoSigner`]; extra keypairs are ignored.
pub fn sign_transaction(
    signers: &[&Keypair],
    ixs: &[Instruction],
    blockhash: &Hash,
) -> Result<SignedTx, Error> {
    let payer = signers.first().ok_or(Error::NoSigner)?.pubkey();
    let (message, required) = compile(&payer, ixs, blockhash);
    let mut sigs = Vec::with_capacity(required.len());
    for key in &required {
        let kp = signers
            .iter()
            .find(|k| k.pubkey() == *key)
            .ok_or(Error::NoSigner)?;
        sigs.push(kp.sign(&message));
    }
    let mut wire = shortvec(sigs.len() as u16);
    for s in &sigs {
        wire.extend_from_slice(&s.0);
    }
    wire.extend_from_slice(&message);
    Ok(SignedTx {
        signature: sigs[0],
        wire,
    })
}
