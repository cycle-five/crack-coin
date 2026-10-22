# crack-coin

CrackCoin is a **worthless-by-design** Solana **devnet** token for the
[CrackTunes](https://cracktun.es/) Discord bot. It has no value and is not meant to.

This crate builds and signs the few transactions the bot needs (create a mint, attach Metaplex
metadata, mint to a member's associated token account) and talks to devnet over raw JSON-RPC.

**It refuses mainnet.** The only way to build the RPC client is `Devnet::connect`, which checks
the cluster's genesis hash and refuses mainnet-beta.

## CLI

Built with the `cli` feature (`cargo run --features cli --bin crack-coin -- <subcommand>`):

- `keygen <path>` creates the devnet authority keypair (mode 0600 file, never overwritten; only
  the public key is printed);
- `airdrop <keypair> [sol=1]` requests devnet SOL for the authority;
- `create-mint <keypair>` creates the CrackCoin mint (decimals 0, no freeze authority) and prints it;
- `set-metadata <keypair> <mint> [uri]` creates or updates the token metadata (name, symbol, uri;
  the uri defaults to the one in `assets/crackcoin.json` on GitHub);
- `info <keypair> <mint>` prints SOL, supply, decimals, whether the authority matches, and the
  on-chain metadata if there is any.

Every subcommand takes `--url <rpc>` (default `https://api.devnet.solana.com`). Minting to members
is done by the bot through the library (`Coin::prepare_mint`), not by the CLI.

The token's logo and metadata JSON are in `assets/`.

## Tests and fixtures

Every encoding test checks the Rust output against `tests/fixtures/vectors.json`, which is
generated from independent JavaScript implementations (`@solana/web3.js`, `@solana/spl-token`,
`@metaplex-foundation/mpl-token-metadata` via umi). To regenerate (deterministic: fixed seeds and
blockhash):

```
cd fixtures/gen && npm ci && node gen.mjs
```

Then `cargo test --all-features`.

## License

MIT
