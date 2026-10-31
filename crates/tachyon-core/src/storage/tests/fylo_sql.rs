use serde_json::{Value, json};

use super::{
    create_fylo_products_collection, execute, fylo_command, fylo_is_available, fylo_save, fylo_url,
};
use crate::storage::ManagedStorage;
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

    fn lexed(sql: &str) -> (Vec<String>, String) {
        let mut strings = Vec::new();
        let mut structure = String::new();
        let mut characters = sql.chars().peekable();
        while let Some(character) = characters.next() {
            if !matches!(character, '\'' | '"') {
                structure.push(character);
                continue;
            }
            let mut text = String::new();
            loop {
                let next = characters.next().expect("terminated string");
                if next != character {
                    text.push(next);
                    continue;
                }
                if characters.next_if_eq(&character).is_none() {
                    break;
                }
                text.push(character);
            }
            structure.push('?');
            strings.push(text);
        }
        (strings, structure)
    }
}

fn rendered(sql: &str, parameters: &[Value]) -> String {
    SqlStatement::rendered(sql, parameters).unwrap_or_else(|error| panic!("{error}"))
}

fn refusal(sql: &str, parameters: &[Value]) -> String {
    SqlStatement::rendered(sql, parameters)
        .err()
        .unwrap_or_else(|| panic!("FYLO SQL accepted {sql}"))
        .to_string()
}

#[test]
fn an_injection_payload_stays_one_string_value_and_leaves_the_statement_unchanged() {
    for payload in FyloSqlProbe::PAYLOADS {
        let sql = rendered("SELECT * FROM products WHERE name = $1", &[json!(payload)]);
        let (strings, structure) = FyloSqlProbe::lexed(&sql);
        assert_eq!(strings, [payload], "{sql}");
        assert_eq!(
            structure, "SELECT * FROM products WHERE name =  ? ",
            "{sql}"
        );
    }
}

#[test]
fn placeholders_in_one_statement_never_reach_each_other() {
    let parameters = [json!("$2' OR '"), json!("' OR name = '$1")];
    let sql = rendered(
        "SELECT * FROM products WHERE name = $1 AND label = $2",
        &parameters,
    );
    let (strings, structure) = FyloSqlProbe::lexed(&sql);
    assert_eq!(strings, ["$2' OR '", "' OR name = '$1"]);
    assert_eq!(
        structure,
        "SELECT * FROM products WHERE name =  ?  AND label =  ? "
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
        assert!(
            refusal(sql, &[json!("x' OR '1'='1")]).contains("outside quotes"),
            "{sql}"
        );
    }
}

#[test]
fn quoted_text_in_the_statement_is_kept_as_written() {
    let sql = rendered(
        "SELECT * FROM products WHERE label = 'US$ it''s \"x\"' AND name = $1",
        &[json!("rope")],
    );
    let (strings, structure) = FyloSqlProbe::lexed(&sql);
    assert_eq!(strings, ["US$ it's \"x\"", "rope"]);
    assert_eq!(
        structure,
        "SELECT * FROM products WHERE label = ? AND name =  ? "
    );
}

#[test]
fn malformed_statements_and_parameters_are_refused() {
    let rope = [json!("rope")];
    assert!(refusal("SELECT * FROM products WHERE name = 'rope", &rope).contains("unterminated"));
    assert!(refusal("SELECT * FROM products WHERE name = $", &rope).contains("$1, $2"));
    assert!(refusal("SELECT * FROM products WHERE name = $0", &rope).contains("out of range"));
    assert!(refusal("SELECT * FROM products WHERE name = $2", &rope).contains("out of range"));
    for number in [json!(-5), json!(-0.5), json!(1e300)] {
        let sql = "SELECT * FROM products WHERE qty > $1";
        assert!(
            refusal(sql, std::slice::from_ref(&number)).contains("non-negative"),
            "{number}"
        );
    }
    assert!(refusal("SELECT * FROM products WHERE qty > $1", &[json!(["x"])]).contains("scalar"));
}

#[test]
fn scalar_parameters_render_as_single_literals() {
    let sql = rendered(
        "SELECT * FROM products WHERE qty>$1 AND price<=$2 AND live=$3 AND gone=$4",
        &[json!(7), json!(2.5), json!(true), Value::Null],
    );
    assert_eq!(
        sql,
        "SELECT * FROM products WHERE qty> 7  AND price<= 2.5  AND live= true  AND gone= NULL "
    );
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
    let store = ManagedStorage::connect(
        &json!({"yon":{"datasource":{"driver":"fylo","url":fylo_url(&canonical)}}}),
    )
    .await
    .unwrap_or_else(|error| panic!("{error}"));
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
    let negative = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE qty > $1","parameters":[-5]});
    assert!(store.execute(&negative).await.is_err());
    let all = execute(&store, fylo_command("find", json!({}))).await;
    assert_eq!(all.as_array().map(Vec::len), Some(2));
    store.close().await;
}
