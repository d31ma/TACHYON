#![allow(clippy::expect_used)]
use serde_json::{Value, json};

use super::{ManagedStorage, dialect::Dialect};

async fn sqlite() -> ManagedStorage {
    ManagedStorage::connect(
        &json!({"yon":{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}}),
    )
    .await
    .unwrap_or_else(|error| panic!("{error}"))
}

async fn execute(store: &ManagedStorage, command: Value) -> Value {
    store
        .execute(&command)
        .await
        .unwrap_or_else(|error| panic!("{error}"))
}

async fn is_rejected(store: &ManagedStorage, command: &Value) -> bool {
    store.execute(command).await.is_err()
}

async fn begin_transaction(store: &ManagedStorage) -> String {
    let opened = execute(store, json!({"operation":"transaction","action":"begin"})).await;
    opened["transaction"]
        .as_str()
        .unwrap_or_default()
        .to_owned()
}

async fn finish_transaction(store: &ManagedStorage, action: &str, id: &str) {
    execute(
        store,
        json!({"operation":"transaction","action":action,"transaction":id}),
    )
    .await;
}

async fn count_items(store: &ManagedStorage) -> Value {
    execute(store, json!({"operation":"query","table":"items","idField":"id","query":{"kind":"count","where":[],"orderBy":[]}})).await
}

fn delete_product(id: i64) -> Value {
    json!({"operation":"delete","table":"products","idField":"id","id":id})
}

fn redis_command(command: &str, key: &str) -> Value {
    json!({"operation":"redis","command":command,"key":key})
}

#[tokio::test]
async fn sqlite_repository_round_trip_and_bound_parameters() {
    let store = sqlite().await;
    execute(&store, json!({"operation":"sql","mode":"execute","parameters":[],"sql":"CREATE TABLE products (id INTEGER PRIMARY KEY, name TEXT, price REAL)"})).await;
    let product = json!({"id":1,"name":"quote'; DROP TABLE products; --","price":12.5});
    let mut save = json!({"operation":"save","table":"products","entity":"ProductEntity","idField":"id","value":product});
    assert_eq!(execute(&store, save.clone()).await, product);
    save["value"]["price"] = json!(13.5);
    execute(&store, save.clone()).await;
    assert_eq!(execute(&store, json!({"operation":"find","table":"products","idField":"id","page":{"page":0,"size":10,"offset":0,"sort":["id:desc"]}})).await, json!([save["value"]]));
    for (kind, expected) in [("count", json!(1)), ("exists", json!(true))] {
        assert_eq!(execute(&store, json!({"operation":"query","table":"products","idField":"id","query":{"kind":kind,"where":[{"field":"name","operator":"eq","value":product["name"]}],"orderBy":[]}})).await, expected);
    }
    assert_eq!(execute(&store, delete_product(1)).await, json!(true));
    assert_eq!(execute(&store, delete_product(1)).await, json!(false));
    store.close().await;
    assert!(is_rejected(&store, &save).await);
}

#[tokio::test]
async fn storage_rejects_oversized_results_and_redacts_errors() {
    let store = sqlite().await;
    let error = store.execute(&json!({"operation":"sql","mode":"rows","parameters":[],"sql":"WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n"})).await.err().unwrap_or_else(|| panic!("oversized result accepted"));
    assert!(error.to_string().contains("row or byte limit"));
    let secret = "sensitive_table_name";
    let error = store.execute(&json!({"operation":"sql","mode":"rows","parameters":[],"sql":format!("SELECT * FROM {secret}")})).await.err().unwrap_or_else(|| panic!("missing table accepted"));
    assert!(!error.to_string().contains(secret));
    store.close().await;
}

#[tokio::test]
async fn sqlite_interrupts_long_queries_and_recovers_its_worker() {
    let store = sqlite().await;
    let started = std::time::Instant::now();
    let command = json!({"operation":"sql","mode":"rows","parameters":[],"sql":"WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n"});
    assert!(is_rejected(&store, &command).await);
    assert!(started.elapsed() < std::time::Duration::from_secs(8));
    assert_eq!(
        execute(
            &store,
            json!({"operation":"sql","mode":"rows","parameters":[],"sql":"SELECT 42 AS answer"})
        )
        .await,
        json!([{"answer":42}])
    );
    assert!(
        tokio::time::timeout(std::time::Duration::from_secs(2), store.close())
            .await
            .is_ok()
    );
}

#[tokio::test]
async fn a_transaction_commits_or_discards_every_statement_in_it() {
    let store = sqlite().await;
    execute(&store, json!({"operation":"sql","mode":"execute","parameters":[],"sql":"CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)"})).await;

    let committed = begin_transaction(&store).await;
    assert!(!committed.is_empty());
    execute(&store, json!({"operation":"save","table":"items","idField":"id","value":{"id":1,"name":"kept"},"transaction":committed})).await;
    finish_transaction(&store, "commit", &committed).await;
    assert_eq!(count_items(&store).await, json!(1));

    let discarded = begin_transaction(&store).await;
    execute(&store, json!({"operation":"save","table":"items","idField":"id","value":{"id":2,"name":"discarded"},"transaction":discarded})).await;
    finish_transaction(&store, "rollback", &discarded).await;
    assert_eq!(
        count_items(&store).await,
        json!(1),
        "a rolled back write must not survive"
    );

    for id in [discarded.as_str(), committed.as_str(), "not-a-transaction"] {
        let commit = json!({"operation":"transaction","action":"commit","transaction":id});
        assert!(is_rejected(&store, &commit).await, "accepted {id}");
        let save = json!({"operation":"save","table":"items","idField":"id","value":{"id":3,"name":"orphan"},"transaction":id});
        assert!(
            is_rejected(&store, &save).await,
            "accepted a command on {id}"
        );
    }
    assert_eq!(count_items(&store).await, json!(1));
}

#[tokio::test]
async fn transaction_control_is_bounded_and_never_authored_as_sql() {
    let store = sqlite().await;
    for command in [
        json!({"operation":"transaction","action":"begin","transaction":"supplied"}),
        json!({"operation":"transaction","action":"savepoint"}),
        json!({"operation":"transaction","action":"commit"}),
        json!({"operation":"transaction"}),
        json!({"operation":"transaction","action":"begin","extra":true}),
        json!({"operation":"sql","mode":"execute","parameters":[],"sql":"BEGIN"}),
        json!({"operation":"sql","mode":"execute","parameters":[],"sql":"COMMIT"}),
        json!({"operation":"save","table":"items","idField":"id","value":{"id":1},"transaction":42}),
    ] {
        assert!(is_rejected(&store, &command).await, "accepted {command}");
    }
}

#[tokio::test]
async fn open_transactions_are_bounded_by_count_and_by_the_connections_there_are() {
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let path = directory.path().join("items.sqlite");
    std::fs::write(&path, []).unwrap_or_else(|error| panic!("{error}"));
    let store = ManagedStorage::connect(&json!({"yon":{"datasource":{"driver":"sqlite","url":format!("sqlite:///{}", path.display().to_string().trim_start_matches('/'))}}}))
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    let mut open = Vec::new();
    for _ in 0..4_u8 {
        open.push(begin_transaction(&store).await);
    }
    let begin = json!({"operation":"transaction","action":"begin"});
    assert!(
        is_rejected(&store, &begin).await,
        "the open transaction limit must hold"
    );
    for id in open {
        finish_transaction(&store, "rollback", &id).await;
    }
    let reopened = begin_transaction(&store).await;
    finish_transaction(&store, "rollback", &reopened).await;
}

#[tokio::test]
async fn the_fylo_datasource_refuses_a_transaction_instead_of_pretending() {
    let store = ManagedStorage::connect(
        &json!({"yon":{"datasource":{"driver":"fylo","url":"/tmp/tachyon-fylo-absent"}}}),
    )
    .await;
    let Ok(store) = store else {
        return;
    };
    let failure = store
        .execute(&json!({"operation":"transaction","action":"begin"}))
        .await
        .expect_err("FYLO has no transactions");
    assert!(
        format!("{failure}").contains("no transactions"),
        "{failure}"
    );
}

#[tokio::test]
async fn startup_is_optional_and_does_not_create_sqlite_files() {
    let empty = ManagedStorage::connect(&json!({}))
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    assert!(is_rejected(&empty, &json!({"operation":"find"})).await);
    let directory = tempfile::tempdir().unwrap_or_else(|error| panic!("{error}"));
    let path = directory.path().join("missing.sqlite");
    assert!(ManagedStorage::connect(&json!({"yon":{"datasource":{"driver":"sqlite","url":format!("sqlite://{}",path.display())}}})).await.is_err());
    assert!(!path.exists());
}

#[tokio::test]
async fn a_delete_query_removes_every_matching_row_and_answers_the_count() {
    let store = sqlite().await;
    execute(&store, json!({"operation":"sql","mode":"execute","parameters":[],"sql":"CREATE TABLE items (id INTEGER PRIMARY KEY, name TEXT)"})).await;
    for (id, name) in [(1, "old"), (2, "old"), (3, "kept")] {
        execute(&store, json!({"operation":"save","table":"items","idField":"id","value":{"id":id,"name":name}})).await;
    }
    let delete_old = json!({"operation":"query","table":"items","idField":"id","query":{"kind":"delete","where":[{"field":"name","operator":"eq","value":"old"}],"orderBy":[]}});
    assert_eq!(execute(&store, delete_old.clone()).await, json!(2));
    assert_eq!(execute(&store, delete_old).await, json!(0));
    assert_eq!(count_items(&store).await, json!(1));
    let paged = json!({"operation":"query","table":"items","idField":"id","query":{"kind":"delete","where":[],"orderBy":[],"page":{"size":1}}});
    assert!(is_rejected(&store, &paged).await);
}

#[test]
fn all_dialects_parameterize_and_quote_identifiers() {
    for (dialect, placeholder, quoted) in [
        (Dialect::Postgres, "$1", "\"name\""),
        (Dialect::Mysql, "?", "`name`"),
        (Dialect::Sqlite, "?", "\"name\""),
        (Dialect::SqlServer, "@P1", "[name]"),
    ] {
        let statement = dialect.compile(&json!({"operation":"query","table":"products","idField":"id","query":{"kind":"find","where":[{"field":"name","operator":"eq","value":"untrusted value"}],"orderBy":[],"page":{"size":2}}})).unwrap_or_else(|error| panic!("{error}"));
        assert!(statement.sql.contains(&format!("{quoted} = {placeholder}")));
        assert!(!statement.sql.contains("untrusted value"));
        assert_eq!(statement.parameters[0], json!("untrusted value"));
        assert!(
            dialect
                .compile(
                    &json!({"operation":"find","table":"products; DROP TABLE users","idField":"id"})
                )
                .is_err()
        );
        if dialect == Dialect::SqlServer {
            assert!(
                statement
                    .sql
                    .contains("OFFSET @P2 ROWS FETCH NEXT @P3 ROWS ONLY")
            );
        }
    }
}

#[tokio::test]
#[ignore = "requires a dedicated Redis instance at YON_TEST_REDIS_URL"]
async fn redis_real_json_ttl_and_delete() {
    let url = std::env::var("YON_TEST_REDIS_URL")
        .unwrap_or_else(|_| panic!("YON_TEST_REDIS_URL is required"));
    let store = ManagedStorage::connect(&json!({"yon":{"redis":{"url":url}}}))
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    let key = format!("tachyon:test:{}", std::process::id());
    assert_eq!(execute(&store, json!({"operation":"redis","command":"set","key":key,"value":{"works":true},"ttlSeconds":30})).await, json!(true));
    let get = redis_command("get", &key);
    assert_eq!(execute(&store, get.clone()).await, json!({"works":true}));
    assert_eq!(
        execute(&store, redis_command("delete", &key)).await,
        json!(true)
    );
    assert_eq!(execute(&store, get).await, json!(null));
    let zero_ttl =
        json!({"operation":"redis","command":"set","key":key,"value":true,"ttlSeconds":0});
    assert!(is_rejected(&store, &zero_ttl).await);
    store.close().await;
}

#[tokio::test]
#[ignore = "requires a dedicated TLS database at YON_TEST_DATASOURCE_URL"]
async fn sql_real_repository_nullable_columns() {
    let value = |key| {
        std::env::var(key).unwrap_or_else(|_| panic!("required database fixture setting missing"))
    };
    let mut settings = json!({"driver":value("YON_TEST_DATASOURCE_DRIVER"),"url":value("YON_TEST_DATASOURCE_URL"),"username":value("YON_TEST_DATASOURCE_USERNAME"),"password":value("YON_TEST_DATASOURCE_PASSWORD")});
    if let Ok(ca) = std::env::var("YON_TEST_DATASOURCE_CA") {
        settings["tls"] = json!({"ca":ca});
    }
    let store = ManagedStorage::connect(&json!({"yon":{"datasource":settings}}))
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    execute(&store, json!({"operation":"sql","mode":"execute","parameters":[],"sql":"CREATE TABLE tachyon_storage_contract (id BIGINT PRIMARY KEY, name VARCHAR(255), optional BIGINT)"})).await;
    let row = json!({"id":42,"name":"bound '; -- value","optional":null});
    let save =
        json!({"operation":"save","table":"tachyon_storage_contract","idField":"id","value":row});
    assert_eq!(execute(&store, save).await, row);
    let rows = execute(&store, json!({"operation":"find","table":"tachyon_storage_contract","idField":"id","page":{"size":5}})).await;
    assert_eq!(rows, json!([row]));
    for (kind, expected) in [("count", json!(1)), ("exists", json!(true))] {
        let result = execute(&store, json!({"operation":"query","table":"tachyon_storage_contract","idField":"id","query":{"kind":kind,"where":[{"field":"id","operator":"gte","value":1}],"orderBy":[]}})).await;
        assert_eq!(result, expected);
    }
    assert_eq!(
        execute(
            &store,
            json!({"operation":"delete","table":"tachyon_storage_contract","idField":"id","id":42})
        )
        .await,
        json!(true)
    );
    execute(&store, json!({"operation":"sql","mode":"execute","parameters":[],"sql":"DROP TABLE tachyon_storage_contract"})).await;
    store.close().await;
}

#[test]
fn predicates_and_save_cover_every_dialect() {
    for dialect in [
        Dialect::Postgres,
        Dialect::Mysql,
        Dialect::Sqlite,
        Dialect::SqlServer,
    ] {
        for (operator, value, sql) in [
            ("eq", json!(4), " = "),
            ("ne", json!(4), " <> "),
            ("lt", json!(4), " < "),
            ("lte", json!(4), " <= "),
            ("gt", json!(4), " > "),
            ("gte", json!(4), " >= "),
            ("eq", Value::Null, " IS NULL"),
            ("ne", Value::Null, " IS NOT NULL"),
            ("isNull", Value::Null, " IS NULL"),
            ("isNotNull", Value::Null, " IS NOT NULL"),
            ("in", json!([1, 2]), " IN ("),
            ("in", json!([]), "1 = 0"),
        ] {
            let statement = dialect.compile(&json!({"operation":"query","table":"items","idField":"id","query":{"kind":"count","where":[{"field":"id","operator":operator,"value":value}],"orderBy":[]}})).unwrap_or_else(|error| panic!("{error}"));
            assert!(statement.sql.contains(sql));
        }
        for value in [
            json!({"id":1}),
            json!({"id":1,"name":null}),
            json!({"id":1,"name":"value"}),
        ] {
            let statement = dialect
                .compile(&json!({"operation":"save","table":"items","idField":"id","value":value}))
                .unwrap_or_else(|error| panic!("{error}"));
            assert!(!statement.sql.contains("'value'"));
            assert!(statement.sql.contains(if dialect == Dialect::SqlServer {
                "MERGE"
            } else {
                "INSERT"
            }));
        }
    }
}

#[test]
fn storage_command_validation_fails_closed() {
    let invalid = [
        json!({"operation":"find","table":"items","idField":"id","page":{"size":0}}),
        json!({"operation":"find","table":"items","idField":"id","page":{"size":101}}),
        json!({"operation":"find","table":"items","idField":"id","page":{"offset":1}}),
        json!({"operation":"find","table":"items","idField":"id","page":{"sort":["id:sideways"]}}),
        json!({"operation":"find","table":"items","idField":"id","extra":true}),
        json!({"operation":"save","table":"items","idField":"id","value":{}}),
        json!({"operation":"save","table":"items","idField":"id","value":{"name":"missing id"}}),
        json!({"operation":"delete","table":"items","idField":"id","id":[]}),
        json!({"operation":"sql","sql":"BEGIN","parameters":[],"mode":"execute"}),
        json!({"operation":"sql","sql":"SELECT 1; SELECT 2","parameters":[],"mode":"rows"}),
        json!({"operation":"sql","sql":"SELECT ?","parameters":[{}],"mode":"rows"}),
        json!({"operation":"sql","sql":"SELECT 1","parameters":[],"mode":"unknown"}),
    ];
    for command in invalid {
        assert!(
            Dialect::Sqlite.compile(&command).is_err(),
            "accepted {command}"
        );
    }
    assert!(Dialect::parse("unknown").is_err());
    assert_eq!(
        Dialect::parse("mariadb").unwrap_or_else(|error| panic!("{error}")),
        Dialect::Mysql
    );
}

fn fylo_is_available() -> bool {
    std::process::Command::new("fylo")
        .arg("version")
        .output()
        .is_ok_and(|output| output.status.success())
}

fn create_fylo_products_collection(root: &std::path::Path) {
    let created = std::process::Command::new("fylo")
        .args([
            "exec",
            "--request",
            r#"{"op":"createCollection","collection":"products"}"#,
            "--root",
        ])
        .arg(root)
        .output()
        .expect("create collection");
    assert!(
        created.status.success(),
        "{}",
        String::from_utf8_lossy(&created.stderr)
    );
}

fn fylo_command(operation: &str, extra: Value) -> Value {
    let mut command = json!({"table":"products","entity":"ProductEntity","idField":"id"});
    command["operation"] = json!(operation);
    let Value::Object(extra) = extra else {
        panic!("extra");
    };
    for (key, value) in extra {
        command[&key] = value;
    }
    command
}

async fn fylo_save(store: &ManagedStorage, value: Value) -> Value {
    execute(store, fylo_command("save", json!({ "value": value }))).await
}

#[tokio::test]
async fn fylo_repository_round_trip() {
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
    let store = ManagedStorage::connect(&json!({"yon":{"datasource":{"driver":"fylo","url":format!("fylo://{}", canonical.display())}}}))
        .await
        .unwrap_or_else(|error| panic!("{error}"));
    let anvil = fylo_save(&store, json!({"name":"anvil","qty":2})).await;
    let id = anvil["id"].as_str().expect("TTID").to_owned();
    fylo_save(&store, json!({"name":"rope","qty":7})).await;
    let patched = fylo_save(&store, json!({"id":id,"name":"anvil","qty":3})).await;
    assert_eq!(patched["qty"], json!(3));
    let page = json!({"page":{"page":0,"size":10,"offset":0,"sort":["qty:desc"]}});
    let all = execute(&store, fylo_command("find", page)).await;
    assert_eq!(all.as_array().map(Vec::len), Some(2));
    assert_eq!(all[0]["name"], json!("rope"));
    let count_query =
        json!({"query":{"kind":"count","where":[{"field":"qty","operator":"gte","value":3}]}});
    let count = execute(&store, fylo_command("query", count_query)).await;
    assert_eq!(count, json!(2));
    let exists = execute(&store, fylo_command("query", json!({"query":{"kind":"exists","where":[{"field":"name","operator":"eq","value":"nobody"}]}}))).await;
    assert_eq!(exists, json!(false));
    let found = execute(&store, fylo_command("query", json!({"query":{"kind":"find","where":[{"field":"name","operator":"in","value":["rope","anvil"]}],"orderBy":["name:asc"]}}))).await;
    assert_eq!(found[0]["name"], json!("anvil"));
    let sql = execute(&store, json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE name = $1","parameters":["rope"]})).await;
    assert_eq!(sql[0]["name"], json!("rope"));
    let delete_rope = json!({"query":{"kind":"delete","where":[{"field":"name","operator":"eq","value":"rope"}]}});
    assert_eq!(
        execute(&store, fylo_command("query", delete_rope)).await,
        json!(1)
    );
    let delete = fylo_command("delete", json!({ "id": id }));
    assert_eq!(execute(&store, delete.clone()).await, json!(true));
    assert_eq!(execute(&store, delete).await, json!(false));
    let ghost = fylo_command("save", json!({"value":{"id":"4W00000000X","name":"ghost"}}));
    assert!(is_rejected(&store, &ghost).await);
    store.close().await;
}

mod fylo_documents;
