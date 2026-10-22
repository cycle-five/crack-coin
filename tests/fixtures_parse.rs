mod common;

#[test]
fn fixtures_parse_and_name_the_four_programs() {
    let v = common::vectors();
    assert_eq!(
        v.programs.token,
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
    );
    assert_eq!(
        v.programs.metadata,
        "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s"
    );
    assert_eq!(v.mint_tx.amount, 500);
    assert!(!v.on_curve.is_empty());
}
