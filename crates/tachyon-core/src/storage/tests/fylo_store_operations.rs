use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use super::{
    begin_transaction, create_fylo_products_collection, execute, fylo_command, fylo_is_available,
    fylo_save, fylo_store, is_rejected,
};
use crate::storage::ManagedStorage;

struct StoreProbe;

impl StoreProbe {
    const WAIT: Duration = Duration::from_millis(300);

    fn runs_here() -> bool {
        let available = fylo_is_available();
        assert!(
            available || std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "fylo binary is required on this runner"
        );
        available
    }

    fn products_root(directory: &tempfile::TempDir) -> PathBuf {
        let root = std::fs::canonicalize(directory.path()).expect("canonical root");
        create_fylo_products_collection(&root);
        root
    }

    fn in_transaction(command: Value, transaction: &str) -> Value {
        let mut command = command;
        command["transaction"] = json!(transaction);
        command
    }

    fn settle(action: &str, transaction: &str) -> Value {
        json!({"operation":"transaction","action":action,"transaction":transaction})
    }

    fn count_where(field: &str, operator: &str, value: &Value) -> Value {
        fylo_command(
            "query",
            json!({"query":{"kind":"count","where":[{"field":field,"operator":operator,"value":value}]}}),
        )
    }

    async fn save_each(store: &ManagedStorage, values: Vec<Value>) {
        for value in values {
            fylo_save(store, value).await;
        }
    }

    async fn refused_in(store: &ManagedStorage, command: Value, transaction: &str) -> bool {
        is_rejected(store, &Self::in_transaction(command, transaction)).await
    }

    async fn count_products(store: &ManagedStorage) -> Value {
        execute(
            store,
            fylo_command("query", json!({"query":{"kind":"count","where":[]}})),
        )
        .await
    }
}

#[tokio::test]
async fn fylo_counts_and_deletes_matches_without_a_scan_in_tachyon() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    let parts = vec![
        json!({"name":"bolt","qty":1,"size":null}),
        json!({"name":"bolt","qty":2,"size":null}),
        json!({"name":"bolt","qty":3,"size":null}),
        json!({"name":"bolt","qty":4,"size":null}),
        json!({"name":"bolt","qty":5,"size":null}),
        json!({"name":"nut","qty":9,"size":"m"}),
    ];
    StoreProbe::save_each(&store, parts).await;
    let gte = execute(&store, StoreProbe::count_where("qty", "gte", &json!(3))).await;
    assert_eq!(gte, json!(4));
    let named = execute(
        &store,
        StoreProbe::count_where("name", "in", &json!(["nut", "washer"])),
    )
    .await;
    assert_eq!(named, json!(1));
    let unsized_parts = fylo_command(
        "query",
        json!({"query":{"kind":"count","where":[{"field":"size","operator":"isNull"}]}}),
    );
    assert_eq!(execute(&store, unsized_parts).await, json!(5));
    let none = fylo_command(
        "query",
        json!({"query":{"kind":"exists","where":[{"field":"name","operator":"in","value":[]}]}}),
    );
    assert_eq!(execute(&store, none).await, json!(false));
    let small = fylo_command(
        "query",
        json!({"query":{"kind":"delete","where":[{"field":"qty","operator":"lt","value":3}]}}),
    );
    assert_eq!(execute(&store, small).await, json!(2));
    assert_eq!(StoreProbe::count_products(&store).await, json!(4));
    store.close().await;
}

#[tokio::test]
async fn fylo_sorts_by_the_query_order_with_the_id_as_tie_breaker() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    let first = fylo_save(&store, json!({"name":"b","qty":1})).await;
    let second = fylo_save(&store, json!({"name":"a","qty":1})).await;
    fylo_save(&store, json!({"name":"c","qty":5})).await;
    let page = json!({"page":{"page":0,"size":2,"offset":0,"sort":["qty:asc"]}});
    let rows = execute(&store, fylo_command("find", page)).await;
    assert_eq!(rows[0]["id"], first["id"]);
    assert_eq!(rows[1]["id"], second["id"]);
    let next = json!({"page":{"page":1,"size":2,"offset":2,"sort":["qty:asc"]}});
    let rows = execute(&store, fylo_command("find", next)).await;
    assert_eq!(rows.as_array().map(Vec::len), Some(1));
    assert_eq!(rows[0]["name"], json!("c"));
    store.close().await;
}

#[tokio::test]
async fn an_entity_with_its_own_id_field_name_keeps_it() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    let save = |value: Value| json!({"operation":"save","table":"products","entity":"PartEntity","idField":"sku","value":value});
    let saved = execute(&store, save(json!({"name":"gear","qty":2}))).await;
    let sku = saved["sku"]
        .as_str()
        .expect("TTID under the entity id field")
        .to_owned();
    assert!(saved.get("id").is_none(), "{saved}");
    let by_sku = json!({"operation":"query","table":"products","entity":"PartEntity","idField":"sku","query":{"kind":"find","where":[{"field":"sku","operator":"eq","value":sku}],"orderBy":["sku:desc"]}});
    let rows = execute(&store, by_sku).await;
    assert_eq!(rows[0]["sku"], json!(sku));
    assert_eq!(rows[0]["name"], json!("gear"));
    store.close().await;
}

#[tokio::test]
async fn a_fylo_transaction_commits_every_write_in_its_collection() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    let transaction = begin_transaction(&store).await;
    let save = fylo_command("save", json!({"value":{"name":"anvil","qty":1}}));
    let saved = execute(&store, StoreProbe::in_transaction(save, &transaction)).await;
    let inside = StoreProbe::in_transaction(
        fylo_command("query", json!({"query":{"kind":"count","where":[]}})),
        &transaction,
    );
    assert_eq!(execute(&store, inside).await, json!(1), "own write");
    assert_eq!(
        StoreProbe::count_products(&store).await,
        json!(0),
        "not yet committed"
    );
    let commit = StoreProbe::settle("commit", &transaction);
    assert_eq!(execute(&store, commit).await["action"], json!("commit"));
    assert_eq!(StoreProbe::count_products(&store).await, json!(1));
    let found = json!({"operation":"document","command":"findById","collection":"products","id":saved["id"]});
    assert_eq!(execute(&store, found).await["name"], json!("anvil"));
    store.close().await;
}

#[tokio::test]
async fn a_rolled_back_fylo_transaction_leaves_no_write() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    let kept = fylo_save(&store, json!({"name":"kept","qty":1})).await;
    let transaction = begin_transaction(&store).await;
    let save = fylo_command("save", json!({"value":{"name":"draft","qty":2}}));
    execute(&store, StoreProbe::in_transaction(save, &transaction)).await;
    let delete = fylo_command("delete", json!({"id":kept["id"]}));
    execute(&store, StoreProbe::in_transaction(delete, &transaction)).await;
    execute(&store, StoreProbe::settle("rollback", &transaction)).await;
    assert_eq!(StoreProbe::count_products(&store).await, json!(1));
    let settled_again = StoreProbe::settle("commit", &transaction);
    assert!(is_rejected(&store, &settled_again).await);
    store.close().await;
}

#[tokio::test]
async fn a_write_to_a_second_collection_ends_the_fylo_transaction() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    execute(
        &store,
        json!({"operation":"document","command":"createCollection","collection":"orders"}),
    )
    .await;
    let transaction = begin_transaction(&store).await;
    let save = fylo_command("save", json!({"value":{"name":"anvil","qty":1}}));
    execute(&store, StoreProbe::in_transaction(save, &transaction)).await;
    let other_read =
        json!({"operation":"document","command":"collectionExists","collection":"orders"});
    let read = StoreProbe::in_transaction(other_read, &transaction);
    assert_eq!(
        execute(&store, read).await,
        json!(true),
        "reads of other collections run outside"
    );
    let order = json!({"operation":"document","command":"insert","collection":"orders","value":{"total":3}});
    let refused = store
        .execute(&StoreProbe::in_transaction(order, &transaction))
        .await
        .expect_err("a second collection is refused");
    assert!(refused.to_string().contains("one collection"), "{refused}");
    assert!(is_rejected(&store, &StoreProbe::settle("commit", &transaction)).await);
    assert_eq!(
        StoreProbe::count_products(&store).await,
        json!(0),
        "rolled back"
    );
    store.close().await;
}

#[tokio::test]
async fn queue_sql_and_collection_commands_do_not_join_a_fylo_transaction() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    let transaction = begin_transaction(&store).await;
    let publish =
        json!({"operation":"queue","command":"publish","topic":"app.render","payload":{"x":1}});
    assert!(
        StoreProbe::refused_in(&store, publish, &transaction).await,
        "queue"
    );
    let sql =
        json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products","parameters":[]});
    assert!(
        StoreProbe::refused_in(&store, sql, &transaction).await,
        "sql"
    );
    let drop = json!({"operation":"document","command":"dropCollection","collection":"products"});
    assert!(
        StoreProbe::refused_in(&store, drop, &transaction).await,
        "drop"
    );
    execute(&store, StoreProbe::settle("rollback", &transaction)).await;
    store.close().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_write_outside_a_fylo_transaction_waits_until_it_settles() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = Arc::new(fylo_store(&StoreProbe::products_root(&directory)).await);
    let transaction = begin_transaction(&store).await;
    let save = fylo_command("save", json!({"value":{"name":"inside","qty":1}}));
    execute(&store, StoreProbe::in_transaction(save, &transaction)).await;
    let outside = Arc::clone(&store);
    let waiting =
        tokio::spawn(async move { fylo_save(&outside, json!({"name":"outside","qty":2})).await });
    tokio::time::sleep(StoreProbe::WAIT).await;
    assert!(
        !waiting.is_finished(),
        "the outside write waits for the transaction"
    );
    execute(&store, StoreProbe::settle("commit", &transaction)).await;
    let written = waiting.await.expect("outside write");
    assert_eq!(written["name"], json!("outside"));
    assert_eq!(StoreProbe::count_products(&store).await, json!(2));
    store.close().await;
}

#[tokio::test]
async fn a_raw_sql_comparison_with_a_null_parameter_matches_no_row() {
    if !StoreProbe::runs_here() {
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let store = fylo_store(&StoreProbe::products_root(&directory)).await;
    fylo_save(&store, json!({"name":null,"qty":1})).await;
    let equal = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE name = $1","parameters":[null]});
    assert_eq!(execute(&store, equal).await, json!([]));
    let unequal = json!({"operation":"sql","mode":"rows","sql":"SELECT * FROM products WHERE name <> $1","parameters":[null]});
    assert_eq!(execute(&store, unequal).await, json!([]));
    store.close().await;
}
