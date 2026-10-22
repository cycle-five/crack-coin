//! A minimal, typed JSON-RPC client for Solana devnet. [`Devnet::connect`] is the only way to
//! get one, and it refuses mainnet.

use std::time::Duration;

use base64::Engine;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::value::RawValue;

use crate::cluster::refuse_mainnet;
use crate::{Error, Hash, Pubkey, Signature};

/// How long any one RPC call may take.
pub const RPC_TIMEOUT: Duration = Duration::from_secs(10);

/// Where a submitted transaction stands.
///
/// `Failed` is reported only when the error is `confirmed` or `finalized`. An error seen at
/// `processed` (or with no confirmation level) can sit on a minority fork, and resending the same
/// bytes may still land, so it is reported as `Processed`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TxStatus {
    /// Confirmed or finalized with no error.
    Landed,
    /// The chain ran it and it errored at `confirmed` or `finalized`; carries the chain's `err`
    /// payload as JSON text.
    Failed(String),
    /// Seen, but not yet confirmed.
    Processed,
    /// The cluster does not know the signature.
    NotFound,
}

// ---- request side -------------------------------------------------------------------------

#[derive(Serialize)]
struct Request<'a, P> {
    jsonrpc: &'a str,
    id: u32,
    method: &'a str,
    params: P,
}

#[derive(Serialize)]
struct Commitment<'a> {
    commitment: &'a str,
}

#[derive(Serialize)]
struct AccountOpts<'a> {
    encoding: &'a str,
    commitment: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SendOpts<'a> {
    encoding: &'a str,
    preflight_commitment: &'a str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct StatusOpts {
    search_transaction_history: bool,
}

fn body<P: Serialize>(method: &str, params: P) -> String {
    let req = Request {
        jsonrpc: "2.0",
        id: 1,
        method,
        params,
    };
    // Serializing plain strings, numbers and bools cannot fail.
    serde_json::to_string(&req).unwrap_or_default()
}

const CONFIRMED: Commitment<'static> = Commitment {
    commitment: "confirmed",
};

pub(crate) fn genesis_request() -> String {
    body("getGenesisHash", [(); 0])
}

pub(crate) fn balance_request(of: &Pubkey) -> String {
    body("getBalance", (of.to_string(), CONFIRMED))
}

pub(crate) fn account_info_request(of: &Pubkey) -> String {
    body(
        "getAccountInfo",
        (
            of.to_string(),
            AccountOpts {
                encoding: "base64",
                commitment: "confirmed",
            },
        ),
    )
}

pub(crate) fn rent_request(size: u64) -> String {
    body("getMinimumBalanceForRentExemption", [size])
}

pub(crate) fn latest_blockhash_request() -> String {
    body("getLatestBlockhash", [CONFIRMED])
}

pub(crate) fn block_height_request() -> String {
    body(
        "getBlockHeight",
        [Commitment {
            commitment: "finalized",
        }],
    )
}

pub(crate) fn send_request(wire: &[u8]) -> String {
    let b64 = base64::engine::general_purpose::STANDARD.encode(wire);
    body(
        "sendTransaction",
        (
            b64,
            SendOpts {
                encoding: "base64",
                preflight_commitment: "confirmed",
            },
        ),
    )
}

pub(crate) fn status_request(sig: &Signature) -> String {
    body(
        "getSignatureStatuses",
        (
            [sig.to_string()],
            StatusOpts {
                search_transaction_history: true,
            },
        ),
    )
}

pub(crate) fn airdrop_request(to: &Pubkey, lamports: u64) -> String {
    body("requestAirdrop", (to.to_string(), lamports, CONFIRMED))
}

// ---- response side ------------------------------------------------------------------------

#[derive(Deserialize)]
struct RpcError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct Response<T> {
    result: Option<T>,
    error: Option<RpcError>,
}

#[derive(Deserialize)]
struct WithValue<T> {
    value: T,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct BlockhashValue {
    blockhash: String,
    last_valid_block_height: u64,
}

#[derive(Deserialize)]
struct AccountValue {
    data: (String, String),
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct StatusValue {
    err: Option<Box<RawValue>>,
    confirmation_status: Option<String>,
}

fn decode_err(what: &str) -> Error {
    Error::Decode(format!("unexpected {what} response"))
}

/// Unwraps the JSON-RPC envelope: an error envelope is `Error::Rpc`, no result is `Decode`.
fn result<T: DeserializeOwned>(text: &str) -> Result<T, Error> {
    let resp: Response<T> = serde_json::from_str(text).map_err(|_| decode_err("rpc"))?;
    if let Some(e) = resp.error {
        return Err(Error::Rpc {
            code: e.code,
            message: e.message,
        });
    }
    resp.result.ok_or_else(|| decode_err("rpc"))
}

pub(crate) fn http_status_error(code: u16) -> String {
    format!("status {code}")
}

pub(crate) fn parse_u64(text: &str) -> Result<u64, Error> {
    result(text)
}

pub(crate) fn parse_balance(text: &str) -> Result<u64, Error> {
    Ok(result::<WithValue<u64>>(text)?.value)
}

pub(crate) fn parse_hash(text: &str) -> Result<Hash, Error> {
    result::<String>(text)?.parse()
}

/// A genesis-hash response, refusing mainnet's.
pub(crate) fn parse_genesis(text: &str) -> Result<Hash, Error> {
    let hash = parse_hash(text)?;
    refuse_mainnet(&hash)?;
    Ok(hash)
}

pub(crate) fn parse_signature(text: &str) -> Result<Signature, Error> {
    result::<String>(text)?.parse()
}

pub(crate) fn parse_latest_blockhash(text: &str) -> Result<(Hash, u64), Error> {
    let v = result::<WithValue<BlockhashValue>>(text)?.value;
    Ok((v.blockhash.parse()?, v.last_valid_block_height))
}

pub(crate) fn parse_account_info(text: &str) -> Result<Option<Vec<u8>>, Error> {
    let Some(v) = result::<WithValue<Option<AccountValue>>>(text)?.value else {
        return Ok(None);
    };
    if v.data.1 != "base64" {
        return Err(decode_err("account encoding"));
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(v.data.0)
        .map_err(|_| decode_err("account data"))?;
    Ok(Some(bytes))
}

pub(crate) fn parse_status(text: &str) -> Result<TxStatus, Error> {
    let mut list = result::<WithValue<Vec<Option<StatusValue>>>>(text)?.value;
    let Some(first) = list.pop() else {
        return Err(decode_err("status"));
    };
    let Some(s) = first else {
        return Ok(TxStatus::NotFound);
    };
    let confirmed = matches!(
        s.confirmation_status.as_deref(),
        Some("confirmed" | "finalized")
    );
    match (s.err, confirmed) {
        (Some(err), true) => Ok(TxStatus::Failed(err.get().to_owned())),
        (None, true) => Ok(TxStatus::Landed),
        (_, false) => Ok(TxStatus::Processed),
    }
}

// ---- the client ---------------------------------------------------------------------------

/// A connection to devnet. There is no other constructor than [`Devnet::connect`].
pub struct Devnet {
    url: String,
    http: reqwest::Client,
}

impl Devnet {
    /// Connects, reads the genesis hash and refuses mainnet.
    pub async fn connect(url: &str) -> Result<(Devnet, Hash), Error> {
        let http = reqwest::Client::builder()
            .timeout(RPC_TIMEOUT)
            .build()
            .map_err(|_| Error::Http("could not build client".into()))?;
        let devnet = Devnet {
            url: url.to_owned(),
            http,
        };
        let genesis = devnet.genesis_hash().await?;
        refuse_mainnet(&genesis)?;
        Ok((devnet, genesis))
    }

    async fn call(&self, request: String) -> Result<String, Error> {
        let resp = self
            .http
            .post(&self.url)
            .header("content-type", "application/json")
            .body(request)
            .send()
            .await
            .map_err(|e| Error::Http(kind(&e)))?;
        let status = resp.status();
        if !status.is_success() {
            return Err(Error::Http(http_status_error(status.as_u16())));
        }
        resp.text().await.map_err(|e| Error::Http(kind(&e)))
    }

    /// The cluster's genesis hash. Returns `Err(Error::Mainnet)` if it is mainnet-beta's.
    pub async fn genesis_hash(&self) -> Result<Hash, Error> {
        parse_genesis(&self.call(genesis_request()).await?)
    }

    pub async fn balance(&self, of: &Pubkey) -> Result<u64, Error> {
        parse_balance(&self.call(balance_request(of)).await?)
    }

    pub async fn account_data(&self, of: &Pubkey) -> Result<Option<Vec<u8>>, Error> {
        parse_account_info(&self.call(account_info_request(of)).await?)
    }

    pub async fn rent_exempt_minimum(&self, size: u64) -> Result<u64, Error> {
        parse_u64(&self.call(rent_request(size)).await?)
    }

    /// The latest confirmed blockhash and the last block height it is valid for.
    pub async fn latest_blockhash(&self) -> Result<(Hash, u64), Error> {
        parse_latest_blockhash(&self.call(latest_blockhash_request()).await?)
    }

    /// The finalized block height.
    ///
    /// Callers judging expiry must read this BEFORE [`Devnet::status`]: a load-balanced RPC pool
    /// can serve a lagging node for one call and a fresh node for the other, so the other order
    /// can show "not found" from a node that is ahead of the height read afterwards. Also apply a
    /// safety margin to the comparison.
    pub async fn finalized_block_height(&self) -> Result<u64, Error> {
        parse_u64(&self.call(block_height_request()).await?)
    }

    /// Submits signed wire bytes, returning the signature the cluster reports.
    pub async fn send(&self, wire: &[u8]) -> Result<Signature, Error> {
        parse_signature(&self.call(send_request(wire)).await?)
    }

    /// Where `sig` stands. Callers judging expiry must read [`Devnet::finalized_block_height`]
    /// BEFORE calling this (see there), and apply a safety margin.
    pub async fn status(&self, sig: &Signature) -> Result<TxStatus, Error> {
        parse_status(&self.call(status_request(sig)).await?)
    }

    pub async fn request_airdrop(&self, to: &Pubkey, lamports: u64) -> Result<Signature, Error> {
        parse_signature(&self.call(airdrop_request(to, lamports)).await?)
    }
}

/// Names the kind of a transport failure without echoing the URL or body.
fn kind(e: &reqwest::Error) -> String {
    if e.is_timeout() {
        "timeout".into()
    } else if e.is_connect() {
        "connect".into()
    } else if e.is_decode() || e.is_body() {
        "body".into()
    } else {
        "request".into()
    }
}

#[cfg(test)]
mod tests;
