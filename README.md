# crack-coin

CrackCoin is a **worthless-by-design** Solana **devnet** token for the
[CrackTunes](https://cracktun.es/) Discord bot. It has no value and is not meant to.

This crate builds and signs the few transactions the bot needs (create a mint, attach Metaplex
metadata, mint to a member's associated token account) and talks to devnet over raw JSON-RPC.

**It refuses mainnet.** The only way to build the RPC client is `Devnet::connect`, which checks
the cluster's genesis hash and refuses mainnet-beta.

## CLI

Built with the `cli` feature (`cargo run --features cli --bin crack-coin -- <subcommand>`):

- `keygen` creates the devnet authority keypair (mode 0600 file; the key is never printed);
- `address` prints the authority's public key;
- `balance` shows the authority's devnet SOL;
- `create-mint` creates the CrackCoin mint (decimals 0, no freeze authority);
- `set-metadata` creates or updates the token metadata (name, symbol, uri);
- `mint` mints an amount to an owner's associated token account.

Environment: `CRACK_COIN_KEYPAIR` (Solana CLI JSON), `CRACK_COIN_MINT` (base58) and the optional
`CRACK_COIN_RPC_URL` (default `https://api.devnet.solana.com`).

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
