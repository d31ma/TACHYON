use serde_json::{Value, json};

use super::{
    create_fylo_products_collection, execute, fylo_command, fylo_is_available, fylo_save,
    fylo_store,
};
use crate::storage::fylo::SqlStatement;

struct FyloSqlProbe;

impl FyloSqlProbe {
    const PAYLOADS: [&'static str; 12] = [
        "x' OR '1'='1",
        "'; DROP COLLECTION products;--",
        "x\" OR name != \"",
        "\\' OR name != '",
        "x\\",
        "\u{2019} OR name != \u{2018}",
        "\u{ff07} OR \u{ff07}1\u{ff07}=\u{ff07}1",
        "\u{2bc} OR name != \u{2bc}",
        "$1 OR name != $2",
        "rope'--",
        "' OR name LIKE '%",
        "rope\u{0} OR 1=1",
    ];
}

fn bound(sql: &str, count: usize) -> String {
    SqlStatement::bound(sql, count).unwrap_or_else(|error| panic!("{error}"))
}

fn refusal(sql: &str, count: usize) -> String {
    SqlStatement::bound(sql, count)
        .err()
        .unwrap_or_else(|| panic!("FYLO SQL accepted {sql}"))
        .to_string()
}

#[test]
fn a_placeholder_becomes_a_numbered_parameter_and_no_value_enters_the_statement() {
    assert_eq!(
        bound("SELECT * FROM products WHERE name = $1", 1),
        "SELECT * FROM products WHERE name = ?1"
    );
}

#[test]
fn every_placeholder_keeps_its_own_number() {
    assert_eq!(
        bound(
            "SELECT * FROM products WHERE name = $1 AND label = $2 OR name = $1",
            2
        ),
        "SELECT * FROM products WHERE name = ?1 AND label = ?2 OR name = ?1"
    );
}

#[test]
fn a_placeholder_inside_quotes_is_refused() {
    for sql in [
        "SELECT * FROM products WHERE name = '$1'",
        "SELECT * FROM products WHERE name = \"$1\"",
        "SELECT * FROM products WHERE name = 'prefix $1 suffix'",
        "SELECT * FROM products WHERE name = 'it''s $1'",
    ] {
        assert!(refusal(sql, 1).contains("outside quotes"), "{sql}");
    }
}

#[test]
fn quoted_text_in_the_statement_is_kept_as_written() {
    assert_eq!(
        bound(
            "SELECT * FROM products WHERE label = 'US$ it''s \"x\" ?' AND name = $1",
            1
        ),
        "SELECT * FROM products WHERE label = 'US$ it''s \"x\" ?' AND name = ?1"
    );
}

#[test]
fn an_unterminated_quote_is_refused() {
    assert!(refusal("SELECT * FROM products WHERE name = 'rope", 1).contains("unterminated"));
}

#[test]
fn a_dollar_sign_without_a_number_is_refused() {
    assert!(refusal("SELECT * FROM products WHERE name = $", 1).contains("$1, $2"));
}

#[test]
fn a_placeholder_number_outside_the_parameters_is_refused() {
    assert!(refusal("SELECT * FROM products WHERE name = $0", 1).contains("out of range"));
    assert!(refusal("SELECT * FROM products WHERE name = $2", 1).contains("out of range"));
}

#[test]
fn a_bare_question_mark_is_refused_because_tachyon_numbers_its_parameters() {
    assert!(refusal("SELECT * FROM products WHERE name = ?", 1).contains("$1, $2"));
}

#[tokio::test]
async fn fylo_answers_injection_payloads_with_only_the_exact_match() {
    if !fylo_is_available() {
        assert!(
            std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "fylo binary is required on this runner"
        );
        return;
    }
    let root = tempfile::tempdir().expect("root");
    let canonical = std::fs::canonicalize(root.path()).expect("canonical");
    create_fylo_products_collection(&canonical);
    let store = fylo_store(&canonical).await;
    let tricky = "O'Brien \"quoted\" \u{2019}curly\u{2019} back\\slash ;--";
    fylo_save(&store, json!({"name":"rope","qty":7})).await;
    fylo_save(&store, json!({"name":tricky,"qty":2})).await;
    let select = |name: Value| json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE name = $1","parameters":[name]});
    for payload in FyloSqlProbe::PAYLOADS {
        let rows = execute(&store, select(json!(payload))).await;
        assert_eq!(rows, json!([]), "{payload}");
    }
    let rows = execute(&store, select(json!(tricky))).await;
    assert_eq!(rows.as_array().map(Vec::len), Some(1));
    assert_eq!(rows[0]["name"], json!(tricky));
    let quoted = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE name = \"$1\"","parameters":["x\" OR name != \""]});
    assert!(store.execute(&quoted).await.is_err());
    let above = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE qty > $1","parameters":[2.5]});
    let rows = execute(&store, above).await;
    assert_eq!(rows.as_array().map(Vec::len), Some(1));
    assert_eq!(rows[0]["name"], json!("rope"));
    let negative = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE qty > $1","parameters":[-5]});
    let rows = execute(&store, negative).await;
    assert_eq!(rows.as_array().map(Vec::len), Some(2));
    let exponent = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE qty < $1","parameters":[1e300]});
    let rows = execute(&store, exponent).await;
    assert_eq!(rows.as_array().map(Vec::len), Some(2));
    let all = execute(&store, fylo_command("find", json!({}))).await;
    assert_eq!(all.as_array().map(Vec::len), Some(2));
    store.close().await;
}
