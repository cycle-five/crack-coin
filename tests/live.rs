//! Talks to the real devnet. Every test is `#[ignore]`; run them with `cargo test -- --ignored`.
//! Nothing here prints key material.

use crack_coin::coin::{wait_for, Coin};
use crack_coin::*;

#[tokio::test]
#[ignore]
async fn connect_refuses_mainnet() {
    let r = Devnet::connect("https://api.mainnet-beta.solana.com").await;
    assert!(matches!(r, Err(Error::Mainnet)));
}

#[tokio::test]
#[ignore]
async fn devnet_connects() {
    let (_, genesis) = Devnet::connect(DEFAULT_RPC_URL).await.unwrap();
    assert_eq!(genesis.to_string(), DEVNET_GENESIS);
}

#[tokio::test]
#[ignore]
async fn mint_round_trip() {
    let keypair = std::env::var("CRACK_COIN_KEYPAIR").expect("CRACK_COIN_KEYPAIR is set");
    let mint: Pubkey = std::env::var("CRACK_COIN_MINT")
        .expect("CRACK_COIN_MINT is set")
        .trim()
        .parse()
        .unwrap();
    let authority = Keypair::from_json(&keypair).unwrap();
    let coin = Coin::open(DEFAULT_RPC_URL, authority, mint).await.unwrap();
    let mut seed = [0u8; 32];
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    seed[..16].copy_from_slice(&t.to_le_bytes());
    let owner = Keypair::from_seed(seed).pubkey();
    let prepared = coin.prepare_mint(&owner, 1).await.unwrap();
    let sent = coin.devnet().send(&prepared.wire).await.unwrap();
    assert_eq!(sent, prepared.signature, "a signature is the tx id");
    let status = wait_for(
        coin.devnet(),
        &prepared.signature,
        prepared.last_valid_block_height,
    )
    .await
    .unwrap();
    assert_eq!(status, TxStatus::Landed);
    assert!(coin.has_token_account(&owner).await.unwrap());
}
