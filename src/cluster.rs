//! Which cluster we are talking to, and the refusal to touch mainnet.

use crate::{Error, Hash};

/// The genesis hash of mainnet-beta. Anything that reports it is refused.
pub const MAINNET_GENESIS: &str = "5eykt4UsFv8P8NJdTREpY1vzqKqZKvdpKuc147dw2N9d";
/// The genesis hash of devnet, the only cluster this crate is for.
pub const DEVNET_GENESIS: &str = "EtWTRABZaYq6iMfeYKouRu166VU2xqa1wcaWoxPkrZBG";
/// Devnet's public RPC endpoint.
pub const DEFAULT_RPC_URL: &str = "https://api.devnet.solana.com";

/// `Err(Error::Mainnet)` when `genesis` is mainnet-beta's.
pub fn refuse_mainnet(genesis: &Hash) -> Result<(), Error> {
    let mainnet: Hash = MAINNET_GENESIS.parse()?;
    if *genesis == mainnet {
        return Err(Error::Mainnet);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Error, Hash};

    #[test]
    fn mainnet_genesis_is_refused_and_devnet_is_not() {
        let main: Hash = MAINNET_GENESIS.parse().unwrap();
        let dev: Hash = DEVNET_GENESIS.parse().unwrap();
        assert!(matches!(refuse_mainnet(&main), Err(Error::Mainnet)));
        assert!(refuse_mainnet(&dev).is_ok());
        assert!(refuse_mainnet(&Hash([0; 32])).is_ok());
    }
}
