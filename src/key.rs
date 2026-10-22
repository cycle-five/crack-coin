//! Keys, signatures and hashes, all displayed as base58.

use std::fmt;
use std::str::FromStr;

use curve25519_dalek::edwards::CompressedEdwardsY;
use ed25519_dalek::{Signer, SigningKey};

use crate::Error;

fn decode<const N: usize>(s: &str) -> Result<[u8; N], Error> {
    let bytes = bs58::decode(s).into_vec().map_err(|_| Error::Base58)?;
    <[u8; N]>::try_from(bytes.as_slice()).map_err(|_| Error::Length {
        expected: N,
        got: bytes.len(),
    })
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Pubkey(pub [u8; 32]);

impl Pubkey {
    /// Whether the bytes decompress to a curve point. Program derived addresses must not.
    pub fn is_on_curve(&self) -> bool {
        CompressedEdwardsY(self.0).decompress().is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Signature(pub [u8; 64]);

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Hash(pub [u8; 32]);

macro_rules! base58 {
    ($t:ident, $n:literal) => {
        impl FromStr for $t {
            type Err = Error;
            fn from_str(s: &str) -> Result<Self, Error> {
                decode::<$n>(s).map($t)
            }
        }
        impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&bs58::encode(self.0).into_string())
            }
        }
    };
}
base58!(Pubkey, 32);
base58!(Signature, 64);
base58!(Hash, 32);

macro_rules! debug_as_display {
    ($($t:ident),*) => {$(
        impl fmt::Debug for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt::Display::fmt(self, f)
            }
        }
    )*};
}
debug_as_display!(Pubkey, Signature, Hash);

/// A signing key. Its `Debug` shows only the public half; it has no `Display` or `Serialize`.
pub struct Keypair(SigningKey);

impl Keypair {
    pub fn from_seed(seed: [u8; 32]) -> Keypair {
        Keypair(SigningKey::from_bytes(&seed))
    }

    /// Parses the Solana CLI format: a JSON array of 64 numbers, `seed ‖ public key`.
    pub fn from_json(text: &str) -> Result<Keypair, Error> {
        let numbers: Vec<u8> = serde_json::from_str(text.trim()).map_err(|_| Error::BadKeypair)?;
        let bytes: [u8; 64] = numbers.try_into().map_err(|_| Error::BadKeypair)?;
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes[..32]);
        let kp = Keypair::from_seed(seed);
        if kp.pubkey().0 != bytes[32..] {
            return Err(Error::BadKeypair);
        }
        Ok(kp)
    }

    /// The CLI JSON text. Only `keygen` should call this.
    pub fn secret_json(&self) -> String {
        let mut all = self.0.to_bytes().to_vec();
        all.extend_from_slice(&self.pubkey().0);
        serde_json::to_string(&all).expect("a byte vector serializes")
    }

    pub fn pubkey(&self) -> Pubkey {
        Pubkey(self.0.verifying_key().to_bytes())
    }

    pub fn sign(&self, msg: &[u8]) -> Signature {
        Signature(self.0.sign(msg).to_bytes())
    }
}

impl fmt::Debug for Keypair {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Keypair({})", self.pubkey())
    }
}
