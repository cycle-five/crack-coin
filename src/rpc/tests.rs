use super::*;

fn pk() -> Pubkey {
    Pubkey([7u8; 32])
}

fn envelope(method: &str, params: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","id":1,"method":"{method}","params":{params}}}"#)
}

#[test]
fn genesis_request_has_empty_params() {
    assert_eq!(genesis_request(), envelope("getGenesisHash", "[]"));
}

#[test]
fn balance_request_exact() {
    assert_eq!(
        balance_request(&pk()),
        envelope(
            "getBalance",
            &format!(r#"["{}",{{"commitment":"confirmed"}}]"#, pk())
        )
    );
}

#[test]
fn account_info_request_exact() {
    assert_eq!(
        account_info_request(&pk()),
        envelope(
            "getAccountInfo",
            &format!(
                r#"["{}",{{"encoding":"base64","commitment":"confirmed"}}]"#,
                pk()
            )
        )
    );
}

#[test]
fn rent_request_exact() {
    assert_eq!(
        rent_request(82),
        envelope("getMinimumBalanceForRentExemption", "[82]")
    );
}

#[test]
fn latest_blockhash_request_exact() {
    assert_eq!(
        latest_blockhash_request(),
        envelope("getLatestBlockhash", r#"[{"commitment":"confirmed"}]"#)
    );
}

#[test]
fn block_height_request_exact() {
    assert_eq!(
        block_height_request(),
        envelope("getBlockHeight", r#"[{"commitment":"finalized"}]"#)
    );
}

#[test]
fn send_request_exact() {
    assert_eq!(
        send_request(&[1, 2, 3]),
        envelope(
            "sendTransaction",
            r#"["AQID",{"encoding":"base64","preflightCommitment":"confirmed"}]"#
        )
    );
}

#[test]
fn status_request_exact() {
    let sig = Signature([9u8; 64]);
    assert_eq!(
        status_request(&sig),
        envelope(
            "getSignatureStatuses",
            &format!(r#"[["{sig}"],{{"searchTransactionHistory":true}}]"#)
        )
    );
}

#[test]
fn airdrop_request_exact() {
    assert_eq!(
        airdrop_request(&pk(), 1_000_000_000),
        envelope(
            "requestAirdrop",
            &format!(r#"["{}",1000000000,{{"commitment":"confirmed"}}]"#, pk())
        )
    );
}

#[test]
fn latest_blockhash_reads_hash_and_last_valid_height() {
    let h = Hash([5u8; 32]);
    let body = format!(
        r#"{{"jsonrpc":"2.0","result":{{"context":{{"slot":2792}},"value":{{"blockhash":"{h}","lastValidBlockHeight":3090}}}},"id":1}}"#
    );
    let (got, height) = parse_latest_blockhash(&body).unwrap();
    assert_eq!(got, h);
    assert_eq!(height, 3090);
}

fn status_body(value: &str) -> String {
    format!(r#"{{"jsonrpc":"2.0","result":{{"context":{{"slot":1}},"value":[{value}]}},"id":1}}"#)
}

#[test]
fn status_null_is_not_found() {
    assert!(matches!(
        parse_status(&status_body("null")).unwrap(),
        TxStatus::NotFound
    ));
}

#[test]
fn status_with_err_is_failed_and_keeps_the_err_text() {
    let body = status_body(
        r#"{"slot":1,"confirmations":null,"err":{"InstructionError":[0,"Custom"]},"confirmationStatus":"confirmed"}"#,
    );
    match parse_status(&body).unwrap() {
        TxStatus::Failed(text) => assert_eq!(text, r#"{"InstructionError":[0,"Custom"]}"#),
        other => panic!("expected Failed, got {other:?}"),
    }
}

#[test]
fn status_confirmed_is_landed() {
    for s in ["confirmed", "finalized"] {
        let body = status_body(&format!(
            r#"{{"slot":1,"confirmations":null,"err":null,"confirmationStatus":"{s}"}}"#
        ));
        assert!(
            matches!(parse_status(&body).unwrap(), TxStatus::Landed),
            "{s}"
        );
    }
}

#[test]
fn status_processed_is_processed() {
    let body =
        status_body(r#"{"slot":1,"confirmations":0,"err":null,"confirmationStatus":"processed"}"#);
    assert!(matches!(parse_status(&body).unwrap(), TxStatus::Processed));
}

#[test]
fn an_rpc_error_envelope_is_error_rpc() {
    let body = r#"{"jsonrpc":"2.0","error":{"code":-32002,"message":"Transaction simulation failed"},"id":1}"#;
    match parse_status(body) {
        Err(Error::Rpc { code, message }) => {
            assert_eq!(code, -32002);
            assert_eq!(message, "Transaction simulation failed");
        },
        other => panic!("expected Rpc error, got {other:?}"),
    }
}

#[test]
fn an_unparseable_body_is_decode() {
    assert!(matches!(parse_status("<html>"), Err(Error::Decode(_))));
}

#[test]
fn account_info_null_value_is_none() {
    let body = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":null},"id":1}"#;
    assert_eq!(parse_account_info(body).unwrap(), None);
}

#[test]
fn account_info_decodes_base64_data() {
    let body = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":{"data":["AQID","base64"],"lamports":5,"owner":"x","executable":false,"rentEpoch":0}},"id":1}"#;
    assert_eq!(parse_account_info(body).unwrap(), Some(vec![1, 2, 3]));
}

#[test]
fn balance_and_numbers_parse() {
    let b = r#"{"jsonrpc":"2.0","result":{"context":{"slot":1},"value":42},"id":1}"#;
    assert_eq!(parse_balance(b).unwrap(), 42);
    assert_eq!(
        parse_u64(r#"{"jsonrpc":"2.0","result":1002240,"id":1}"#).unwrap(),
        1_002_240
    );
}

#[test]
fn http_errors_carry_only_the_status() {
    assert_eq!(http_status_error(429), "status 429");
}
