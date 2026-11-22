use serde_json::{Value, json};

use super::{begin_transaction, execute, fylo_is_available, fylo_store, sqlite};
use crate::resource_error_code::ResourceErrorCode;
use crate::storage::ManagedStorage;

async fn failure_code(store: &ManagedStorage, command: &Value) -> ResourceErrorCode {
    store
        .execute(command)
        .await
        .expect_err("the command must fail")
        .resource_code()
}

fn sql(mode: &str, statement: &str) -> Value {
    json!({"operation":"sql","mode":mode,"parameters":[],"sql":statement})
}

async fn file_sqlite(directory: &tempfile::TempDir) -> ManagedStorage {
    let path = directory.path().join("items.sqlite");
    std::fs::write(&path, []).expect("database file");
    let url = format!(
        "sqlite:///{}",
        path.display().to_string().trim_start_matches('/')
    );
    ManagedStorage::connect(&json!({"yon":{"datasource":{"driver":"sqlite","url":url}}}))
        .await
        .expect("file storage")
}

#[tokio::test]
async fn a_duplicate_key_is_a_conflict() {
    let store = sqlite().await;
    execute(
        &store,
        sql("execute", "CREATE TABLE items (id INTEGER PRIMARY KEY)"),
    )
    .await;
    let insert = sql("execute", "INSERT INTO items (id) VALUES (1)");
    execute(&store, insert.clone()).await;
    assert_eq!(
        failure_code(&store, &insert).await,
        ResourceErrorCode::Conflict
    );
    store.close().await;
}

#[tokio::test]
async fn a_result_over_the_limit_is_too_large() {
    let store = sqlite().await;
    let rows = sql(
        "rows",
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n WHERE x < 101) SELECT x FROM n",
    );
    assert_eq!(
        failure_code(&store, &rows).await,
        ResourceErrorCode::TooLarge
    );
    store.close().await;
}

#[tokio::test]
async fn a_command_over_the_limit_is_too_large() {
    let store = sqlite().await;
    let padding = "x".repeat(1024 * 1024);
    let command = json!({"operation":"sql","mode":"rows","parameters":[padding],"sql":"SELECT ?"});
    assert_eq!(
        failure_code(&store, &command).await,
        ResourceErrorCode::TooLarge
    );
    store.close().await;
}

#[tokio::test]
async fn an_unknown_transaction_is_refused() {
    let store = sqlite().await;
    let commit = json!({"operation":"transaction","action":"commit","transaction":"missing"});
    assert_eq!(
        failure_code(&store, &commit).await,
        ResourceErrorCode::TransactionRefused
    );
    store.close().await;
}

#[tokio::test]
async fn a_transaction_over_the_open_limit_is_refused() {
    let directory = tempfile::tempdir().expect("directory");
    let store = file_sqlite(&directory).await;
    for _ in 0..4_u8 {
        begin_transaction(&store).await;
    }
    let begin = json!({"operation":"transaction","action":"begin"});
    assert_eq!(
        failure_code(&store, &begin).await,
        ResourceErrorCode::TransactionRefused
    );
    store.close().await;
}

#[tokio::test]
async fn a_query_stopped_by_its_deadline_spent_the_budget() {
    let store = sqlite().await;
    let endless = sql(
        "rows",
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x+1 FROM n) SELECT sum(x) FROM n",
    );
    assert_eq!(
        failure_code(&store, &endless).await,
        ResourceErrorCode::BudgetSpent
    );
    store.close().await;
}

#[tokio::test]
async fn a_command_after_the_shared_deadline_spent_the_budget() {
    let store = sqlite().await;
    let past = std::time::Instant::now();
    let failure = store
        .execute_within(&sql("rows", "SELECT 1"), Some(past))
        .await
        .expect_err("a call after its deadline must fail");
    assert_eq!(failure.resource_code(), ResourceErrorCode::BudgetSpent);
    store.close().await;
}

#[tokio::test]
async fn an_invalid_statement_is_failed() {
    let store = sqlite().await;
    let missing = sql("rows", "SELECT * FROM private_table_name");
    assert_eq!(
        failure_code(&store, &missing).await,
        ResourceErrorCode::Failed
    );
    store.close().await;
}

#[tokio::test]
async fn a_write_to_a_missing_fylo_collection_is_not_found() {
    if !fylo_is_available() {
        assert!(
            std::env::var("TAC_STORAGE_REQUIRE_TOOLCHAINS").is_err(),
            "fylo binary is required on this runner"
        );
        return;
    }
    let directory = tempfile::tempdir().expect("root");
    let root = std::fs::canonicalize(directory.path()).expect("canonical root");
    let store = fylo_store(&root).await;
    let insert = json!({"operation":"document","command":"insert","collection":"missing-things","value":{"name":"a"}});
    assert_eq!(
        failure_code(&store, &insert).await,
        ResourceErrorCode::NotFound
    );
    store.close().await;
}
