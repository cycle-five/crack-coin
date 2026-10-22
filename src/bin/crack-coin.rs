//! The CrackCoin operator CLI. It prints only public data: keys, signatures, balances and
//! metadata. The signing key is read from a file and never shown.

use std::path::Path;
use std::process::ExitCode;

use crack_coin::coin::{
    check_mint, create_mint, read_metadata, set_metadata, wait_for, write_keypair_file,
};
use crack_coin::{
    metadata_address, Devnet, Error, Keypair, MintAccount, Pubkey, TokenInfo, TxStatus,
    DEFAULT_RPC_URL, METADATA_URI, NAME, SYMBOL,
};

const USAGE: &str = "usage:
  crack-coin keygen <path>
  crack-coin airdrop <keypair> [sol=1]
  crack-coin create-mint <keypair>
  crack-coin set-metadata <keypair> <mint> [uri]
  crack-coin info <keypair> <mint>
every subcommand also takes --url <rpc> (default https://api.devnet.solana.com)";

const LAMPORTS_PER_SOL: f64 = 1_000_000_000.0;

fn load_keypair(path: &str) -> Result<Keypair, Error> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::Io(e.kind().to_string()))?;
    Keypair::from_json(&text)
}

/// Splits `--url <rpc>` out of the arguments.
fn split_url(args: Vec<String>) -> Result<(String, Vec<String>), String> {
    let mut url = DEFAULT_RPC_URL.to_owned();
    let mut rest = Vec::new();
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        if a == "--url" {
            url = it.next().ok_or("--url needs a value")?;
        } else {
            rest.push(a);
        }
    }
    Ok((url, rest))
}

fn parse_pubkey(s: &str) -> Result<Pubkey, String> {
    s.parse::<Pubkey>().map_err(|e| format!("bad mint: {e}"))
}

async fn run(args: Vec<String>) -> Result<(), String> {
    let (url, args) = split_url(args)?;
    let mut it = args.iter().map(String::as_str);
    let cmd = it.next().ok_or(USAGE)?;
    let rest: Vec<&str> = it.collect();
    match (cmd, rest.as_slice()) {
        ("keygen", [path]) => {
            let mut seed = [0u8; 32];
            getrandom::fill(&mut seed).map_err(|_| "no randomness available".to_string())?;
            let kp = Keypair::from_seed(seed);
            write_keypair_file(Path::new(path), &kp).map_err(|e| e.to_string())?;
            println!("{}", kp.pubkey());
        },
        ("airdrop", [keypair, tail @ ..]) if tail.len() <= 1 => {
            let sol: f64 = match tail.first() {
                Some(s) => s.parse().map_err(|_| "sol must be a number".to_string())?,
                None => 1.0,
            };
            if !sol.is_finite() || sol <= 0.0 {
                return Err("sol must be a positive number".into());
            }
            let kp = load_keypair(keypair).map_err(|e| e.to_string())?;
            let (devnet, _) = Devnet::connect(&url).await.map_err(|e| e.to_string())?;
            let (_, last_valid) = devnet.latest_blockhash().await.map_err(|e| e.to_string())?;
            let sig = devnet
                .request_airdrop(&kp.pubkey(), (sol * LAMPORTS_PER_SOL) as u64)
                .await
                .map_err(|e| e.to_string())?;
            let status = wait_for(&devnet, &sig, last_valid)
                .await
                .map_err(|e| e.to_string())?;
            println!("{sig} {status:?}");
            if status != TxStatus::Landed {
                return Err("airdrop did not land".into());
            }
        },
        ("create-mint", [keypair]) => {
            let kp = load_keypair(keypair).map_err(|e| e.to_string())?;
            let (devnet, _) = Devnet::connect(&url).await.map_err(|e| e.to_string())?;
            let mut seed = [0u8; 32];
            getrandom::fill(&mut seed).map_err(|_| "no randomness available".to_string())?;
            let mint = Keypair::from_seed(seed);
            let sig = create_mint(&devnet, &kp, &mint)
                .await
                .map_err(|e| e.to_string())?;
            println!("mint {}", mint.pubkey());
            println!("signature {sig}");
        },
        ("set-metadata", [keypair, mint, tail @ ..]) if tail.len() <= 1 => {
            let kp = load_keypair(keypair).map_err(|e| e.to_string())?;
            let mint = parse_pubkey(mint)?;
            let info = TokenInfo {
                name: NAME,
                symbol: SYMBOL,
                uri: tail.first().copied().unwrap_or(METADATA_URI),
            };
            let (devnet, _) = Devnet::connect(&url).await.map_err(|e| e.to_string())?;
            let sig = set_metadata(&devnet, &kp, &mint, &info)
                .await
                .map_err(|e| e.to_string())?;
            println!("signature {sig}");
        },
        ("info", [keypair, mint]) => {
            let kp = load_keypair(keypair).map_err(|e| e.to_string())?;
            let mint = parse_pubkey(mint)?;
            let (devnet, _) = Devnet::connect(&url).await.map_err(|e| e.to_string())?;
            let lamports = devnet
                .balance(&kp.pubkey())
                .await
                .map_err(|e| e.to_string())?;
            println!("authority {}", kp.pubkey());
            println!("sol {:.4}", lamports as f64 / LAMPORTS_PER_SOL);
            let data = devnet
                .account_data(&mint)
                .await
                .map_err(|e| e.to_string())?;
            match check_mint(data.as_deref(), &kp.pubkey()) {
                Ok(MintAccount {
                    supply, decimals, ..
                }) => {
                    println!("supply {supply}");
                    println!("decimals {decimals}");
                    println!("authority matches yes");
                },
                Err(e) => println!("mint check: {e}"),
            }
            match devnet
                .account_data(&metadata_address(&mint))
                .await
                .map_err(|e| e.to_string())?
            {
                None => println!("no metadata"),
                Some(raw) => match read_metadata(&raw) {
                    Ok((name, symbol, uri)) => {
                        println!("name {name}");
                        println!("symbol {symbol}");
                        println!("uri {uri}");
                    },
                    Err(_) => println!("no metadata"),
                },
            }
        },
        _ => return Err(USAGE.into()),
    }
    Ok(())
}

#[tokio::main]
async fn main() -> ExitCode {
    match run(std::env::args().skip(1).collect()).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        },
    }
}
