use std::str::FromStr;

use futures_util::TryStreamExt;
use redis::aio::ConnectionManager;
use serde_json::{Map, Value, json};
use sqlx::{Column, ConnectOptions, Row, TypeInfo, ValueRef};
#[cfg(feature = "storage-sqlserver")]
use tokio::net::TcpStream;
use tokio::sync::Mutex;
#[cfg(feature = "storage-sqlserver")]
use tokio_util::compat::{Compat, TokioAsyncWriteCompatExt};

use super::dialect::{Dialect, ResultKind, Statement};
#[cfg(feature = "storage-sqlserver")]
use super::prelogin_stream::PreloginStream;
use super::{ManagedStorage, failure, object_keys, text};
use crate::Failure;

#[cfg(feature = "storage-sqlserver")]
type TdsClient = tiberius::Client<PreloginStream<Compat<TcpStream>>>;

pub(super) struct SqlStore {
    dialect: Dialect,
    connection: SqlConnection,
    transactions: Mutex<std::collections::BTreeMap<String, OpenTransaction>>,
}

struct OpenTransaction {
    connection: DedicatedConnection,
    expires: std::time::Instant,
}

enum DedicatedConnection {
    Pool(sqlx::pool::PoolConnection<sqlx::Any>),
    Sqlite(sqlx::pool::PoolConnection<sqlx::Sqlite>),
    #[cfg(feature = "storage-sqlserver")]
    Tds(Box<TdsClient>),
}

enum SqlConnection {
    Pool(sqlx::AnyPool),
    Sqlite(super::sqlite::SqliteStore),
    #[cfg(feature = "storage-sqlserver")]
    Tds(Box<TdsStore>),
}

#[cfg(feature = "storage-sqlserver")]
struct TdsStore {
    config: tiberius::Config,
    connection: Mutex<Option<TdsClient>>,
}

impl SqlStore {
    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["driver", "url", "username", "password", "tls"])?;
        let dialect = Dialect::parse(text(settings, "driver")?)?;
        if dialect != Dialect::Sqlite {
            require_network_credentials(settings)?;
        }
        if dialect == Dialect::Postgres {
            reject_postgres_environment()?;
        }
        let raw = text(settings, "url")?;
        let connection = match dialect {
            Dialect::Sqlite => connect_sqlite(settings, raw).await?,
            Dialect::SqlServer => connect_sql_server(settings).await?,
            Dialect::Postgres | Dialect::Mysql => connect_pool(settings, dialect).await?,
        };
        Ok(Self {
            dialect,
            connection,
            transactions: Mutex::new(std::collections::BTreeMap::new()),
        })
    }

    pub(super) async fn begin(&self) -> Result<Value, Failure> {
        let mut open = self.transactions.lock().await;
        Self::remove_expired(&mut open);
        if open.len() >= ManagedStorage::MAX_TRANSACTIONS {
            return Err(failure(
                "Open transaction limit reached; commit or roll back before starting another.",
            ));
        }
        let mut connection = self.dedicated_connection().await?;
        run_on_dedicated(&mut connection, &self.dialect.control("begin")).await?;
        let id = crate::ttid::generate();
        open.insert(
            id.clone(),
            OpenTransaction {
                connection,
                expires: std::time::Instant::now() + ManagedStorage::TRANSACTION_TIMEOUT,
            },
        );
        Ok(json!({ "transaction": id }))
    }

    pub(super) async fn execute_in(&self, id: &str, command: &Value) -> Result<Value, Failure> {
        let statement = self.dialect.compile(command)?;
        let mut transaction = self.take_transaction(id).await?;
        let result = run_on_dedicated(&mut transaction.connection, &statement).await;
        if result.is_ok() {
            transaction.expires = std::time::Instant::now() + ManagedStorage::TRANSACTION_TIMEOUT;
            self.transactions
                .lock()
                .await
                .insert(id.to_owned(), transaction);
        } else {
            let _ = run_on_dedicated(
                &mut transaction.connection,
                &self.dialect.control("rollback"),
            )
            .await;
        }
        result
    }

    pub(super) async fn settle(&self, id: &str, action: &str) -> Result<Value, Failure> {
        let mut transaction = self.take_transaction(id).await?;
        let result =
            run_on_dedicated(&mut transaction.connection, &self.dialect.control(action)).await;
        if action == "commit" && result.is_err() {
            let _ = run_on_dedicated(
                &mut transaction.connection,
                &self.dialect.control("rollback"),
            )
            .await;
        }
        result.map(|_| json!({ "transaction": id, "action": action }))
    }

    async fn take_transaction(&self, id: &str) -> Result<OpenTransaction, Failure> {
        let mut open = self.transactions.lock().await;
        Self::remove_expired(&mut open);
        open.remove(id).ok_or_else(|| {
            failure("Unknown, expired or already settled transaction; start a new one.")
        })
    }

    fn remove_expired(open: &mut std::collections::BTreeMap<String, OpenTransaction>) {
        let now = std::time::Instant::now();
        open.retain(|_, transaction| transaction.expires > now);
    }

    async fn dedicated_connection(&self) -> Result<DedicatedConnection, Failure> {
        match &self.connection {
            SqlConnection::Pool(pool) => pool
                .acquire()
                .await
                .map(DedicatedConnection::Pool)
                .map_err(|_| failure("Datasource connection admission failed.")),
            SqlConnection::Sqlite(store) => store
                .acquire_connection()
                .await
                .map(DedicatedConnection::Sqlite),
            #[cfg(feature = "storage-sqlserver")]
            SqlConnection::Tds(store) => {
                let mut guard = store.connection.lock().await;
                let client = match guard.take() {
                    Some(client) => client,
                    None => connect_tds(&store.config).await?,
                };
                Ok(DedicatedConnection::Tds(Box::new(client)))
            }
        }
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        let statement = self.dialect.compile(command)?;
        match &self.connection {
            SqlConnection::Pool(pool) => execute_sqlx(pool, &statement).await,
            SqlConnection::Sqlite(store) => store.execute(&statement).await,
            #[cfg(feature = "storage-sqlserver")]
            SqlConnection::Tds(store) => {
                let mut guard = store.connection.lock().await;
                let mut client = match guard.take() {
                    Some(client) => client,
                    None => connect_tds(&store.config).await?,
                };
                let result = execute_tds(&mut client, &statement).await;
                if result.is_ok() && !statement.isolated {
                    *guard = Some(client);
                }
                result
            }
        }
    }

    pub(super) async fn close(&self) {
        self.transactions.lock().await.clear();
        match &self.connection {
            SqlConnection::Pool(pool) => pool.close().await,
            SqlConnection::Sqlite(store) => store.close().await,
            #[cfg(feature = "storage-sqlserver")]
            SqlConnection::Tds(store) => {
                store.connection.lock().await.take();
            }
        }
    }
}

fn require_network_credentials(settings: &Value) -> Result<(), Failure> {
    text(settings, "username")?;
    if settings.get("password").and_then(Value::as_str).is_none() {
        return Err(failure(
            "Network datasource credentials must be supplied through project .env.",
        ));
    }
    Ok(())
}

async fn connect_sqlite(settings: &Value, raw: &str) -> Result<SqlConnection, Failure> {
    validate_sqlite(raw)?;
    let has_network_settings = ["username", "password", "tls"]
        .iter()
        .any(|key| settings.get(key).is_some());
    if has_network_settings {
        return Err(failure(
            "SQLite does not accept network credentials or TLS settings.",
        ));
    }
    let store = super::sqlite::SqliteStore::connect(raw).await?;
    Ok(SqlConnection::Sqlite(store))
}

#[cfg(feature = "storage-sqlserver")]
async fn connect_sql_server(settings: &Value) -> Result<SqlConnection, Failure> {
    let url = connection_url(settings, "sqlserver")?;
    let host = url
        .host_str()
        .ok_or_else(|| failure("SQL Server requires a host."))?;
    let mut config = tiberius::Config::new();
    config.host(host);
    config.port(url.port().unwrap_or(1433));
    config.database(url.path().trim_start_matches('/'));
    config.authentication(tiberius::AuthMethod::sql_server(
        settings["username"].as_str().unwrap_or(""),
        settings["password"].as_str().unwrap_or(""),
    ));
    config.encryption(tiberius::EncryptionLevel::Required);
    if let Some(certificate_path) = certificate(settings)? {
        config.trust_cert_ca(certificate_path);
    }
    let client = connect_tds(&config).await?;
    Ok(SqlConnection::Tds(Box::new(TdsStore {
        config,
        connection: Mutex::new(Some(client)),
    })))
}

#[cfg(not(feature = "storage-sqlserver"))]
fn connect_sql_server(_settings: &Value) -> std::future::Ready<Result<SqlConnection, Failure>> {
    std::future::ready(Err(failure(
        "This ty was built without SQL Server support; rebuild it with `--features storage-sqlserver` (release builds include it).",
    )))
}

async fn connect_pool(settings: &Value, dialect: Dialect) -> Result<SqlConnection, Failure> {
    let scheme = if dialect == Dialect::Postgres {
        "postgresql"
    } else {
        "mysql"
    };
    let url = connection_url(settings, scheme)?.to_string();
    sqlx::any::install_default_drivers();
    let options = sqlx::any::AnyConnectOptions::from_str(&url)
        .map_err(|_| failure("Invalid datasource connection URL."))?
        .disable_statement_logging();
    let pool = sqlx::any::AnyPoolOptions::new()
        .max_connections(8)
        .idle_timeout(None)
        .max_lifetime(None)
        .acquire_timeout(ManagedStorage::OPERATION_TIMEOUT)
        .connect_with(options)
        .await
        .map_err(|_| {
            failure("Datasource connection failed; inspect database availability and credentials.")
        })?;
    Ok(SqlConnection::Pool(pool))
}

async fn run_on_dedicated(
    connection: &mut DedicatedConnection,
    statement: &Statement,
) -> Result<Value, Failure> {
    match connection {
        DedicatedConnection::Pool(connection) => execute_on(connection, statement).await,
        DedicatedConnection::Sqlite(connection) => {
            super::sqlite::SqliteStore::execute_on(connection, statement).await
        }
        #[cfg(feature = "storage-sqlserver")]
        DedicatedConnection::Tds(client) => execute_tds(client, statement).await,
    }
}

fn validate_sqlite(url: &str) -> Result<(), Failure> {
    if url == "sqlite::memory:" {
        return Ok(());
    }
    if !url.starts_with("sqlite:///")
        || url.contains('?')
        || url.contains('#')
        || url.contains('\0')
    {
        return Err(failure(
            "SQLite requires sqlite::memory: or an absolute sqlite:/// path to an existing database; URL options are not allowed.",
        ));
    }
    Ok(())
}

fn connection_url(settings: &Value, scheme: &str) -> Result<url::Url, Failure> {
    let mut url = url::Url::parse(text(settings, "url")?)
        .map_err(|_| failure("Invalid storage connection URL."))?;
    let postgres_alias = scheme == "postgresql" && url.scheme() == "postgres";
    let mariadb_alias =
        scheme == "mysql" && url.scheme() == "mariadb" && settings["driver"] == "mariadb";
    if postgres_alias || mariadb_alias {
        url.set_scheme(scheme)
            .map_err(|()| failure("Invalid storage URL scheme."))?;
    }
    if !is_plain_url_for_scheme(&url, scheme) {
        return Err(failure(
            "Storage URL requires the matching scheme, a host, and no credentials or URL options.",
        ));
    }
    apply_credentials(&mut url, settings)?;
    if scheme == "postgresql" {
        apply_postgres_options(&mut url, settings)?;
    }
    if scheme == "mysql" {
        apply_mysql_options(&mut url, settings)?;
    }
    Ok(url)
}

fn is_plain_url_for_scheme(url: &url::Url, scheme: &str) -> bool {
    url.scheme() == scheme
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
        && url.host_str().is_some()
}

fn apply_credentials(url: &mut url::Url, settings: &Value) -> Result<(), Failure> {
    for key in ["username", "password"] {
        let Some(value) = settings.get(key) else {
            continue;
        };
        let value = value
            .as_str()
            .filter(|value| value.len() <= 65536)
            .ok_or_else(|| failure("Storage credentials must be bounded strings."))?;
        if key == "username" {
            url.set_username(value)
                .map_err(|()| failure("Invalid storage credentials."))?;
        } else {
            url.set_password(Some(value))
                .map_err(|()| failure("Invalid storage credentials."))?;
        }
    }
    Ok(())
}

fn apply_postgres_options(url: &mut url::Url, settings: &Value) -> Result<(), Failure> {
    validate_database_path(url)?;
    if url.port().is_none() {
        url.set_port(Some(5432))
            .map_err(|()| failure("Invalid datasource port."))?;
    }
    url.query_pairs_mut().append_pair("sslmode", "verify-full");
    url.query_pairs_mut()
        .append_pair("options", "-c statement_timeout=5000 -c lock_timeout=5000");
    if let Some(certificate_path) = certificate(settings)? {
        url.query_pairs_mut()
            .append_pair("sslrootcert", certificate_path);
    }
    Ok(())
}

fn apply_mysql_options(url: &mut url::Url, settings: &Value) -> Result<(), Failure> {
    validate_database_path(url)?;
    url.query_pairs_mut()
        .append_pair("ssl-mode", "VERIFY_IDENTITY");
    if let Some(certificate_path) = certificate(settings)? {
        url.query_pairs_mut()
            .append_pair("ssl-ca", certificate_path);
    }
    Ok(())
}

fn validate_database_path(url: &url::Url) -> Result<(), Failure> {
    if url.path().trim_start_matches('/').is_empty() {
        return Err(failure("Network datasource URL must select a database."));
    }
    Ok(())
}

fn reject_postgres_environment() -> Result<(), Failure> {
    const KEYS: [&str; 12] = [
        "PGPORT",
        "PGHOSTADDR",
        "PGHOST",
        "PGUSER",
        "PGDATABASE",
        "PGPASSWORD",
        "PGSSLROOTCERT",
        "PGSSLCERT",
        "PGSSLKEY",
        "PGSSLMODE",
        "PGAPPNAME",
        "PGOPTIONS",
    ];
    if KEYS.iter().any(|key| std::env::var_os(key).is_some()) {
        return Err(failure(
            "Ambient PostgreSQL settings are not supported; use Yon application configuration and root .env.",
        ));
    }
    Ok(())
}

fn certificate(settings: &Value) -> Result<Option<&str>, Failure> {
    let Some(tls) = settings.get("tls") else {
        return Ok(None);
    };
    object_keys(tls, &["ca"])?;
    let ca = text(tls, "ca")?;
    if !std::path::Path::new(ca).is_absolute() || ca.contains('\0') {
        return Err(failure(
            "Datasource CA certificate requires an absolute path.",
        ));
    }
    Ok(Some(ca))
}

#[cfg(feature = "storage-sqlserver")]
async fn connect_tds(config: &tiberius::Config) -> Result<TdsClient, Failure> {
    let stream = TcpStream::connect(config.get_addr())
        .await
        .map_err(|_| failure("SQL Server connection failed."))?;
    stream
        .set_nodelay(true)
        .map_err(|_| failure("SQL Server socket setup failed."))?;
    tiberius::Client::connect(config.clone(), PreloginStream::new(stream.compat_write()))
        .await
        .map_err(|_| failure("SQL Server TLS or authentication failed."))
}

async fn execute_sqlx(pool: &sqlx::AnyPool, statement: &Statement) -> Result<Value, Failure> {
    let mut connection = pool
        .acquire()
        .await
        .map_err(|_| failure("Datasource connection admission failed."))?;
    if statement.isolated {
        connection.close_on_drop();
    }
    execute_on(&mut connection, statement).await
}

async fn execute_on(
    connection: &mut sqlx::AnyConnection,
    statement: &Statement,
) -> Result<Value, Failure> {
    let mut query = sqlx::query(sqlx::AssertSqlSafe(statement.sql.clone()));
    for parameter in &statement.parameters {
        query = match parameter {
            Value::Null => query.bind(Option::<String>::None),
            Value::Bool(value) => query.bind(*value),
            Value::Number(value) if value.is_i64() => query.bind(value.as_i64()),
            Value::Number(value) => query.bind(value.as_f64()),
            Value::String(value) => query.bind(value.clone()),
            _ => return Err(failure("Invalid SQL parameter.")),
        };
    }
    if !statement.result.returns_rows() {
        let result = query
            .execute(&mut *connection)
            .await
            .map_err(|_| failure("Datasource mutation failed; no retry was performed."))?;
        return Ok(mutation_result(&statement.result, result.rows_affected()));
    }
    let mut stream = query.fetch(&mut *connection);
    let mut rows = Vec::new();
    let mut bytes = 0;
    while let Some(row) = stream
        .try_next()
        .await
        .map_err(|_| failure("Datasource query failed."))?
    {
        push_row(&mut rows, sqlx_row(&row)?, &mut bytes)?;
    }
    rows_result(&statement.result, rows)
}

fn sqlx_row(row: &sqlx::any::AnyRow) -> Result<Value, Failure> {
    let mut object = Map::new();
    if row.columns().len() > 64 {
        return Err(failure("Storage row column limit exceeded."));
    }
    for (index, column) in row.columns().iter().enumerate() {
        if object.contains_key(column.name()) {
            return Err(failure("SQL result column names must be unique."));
        }
        let value = sqlx_column_json(row, index)?;
        object.insert(column.name().into(), value);
    }
    Ok(Value::Object(object))
}

fn sqlx_column_json(row: &sqlx::any::AnyRow, index: usize) -> Result<Value, Failure> {
    let raw = row
        .try_get_raw(index)
        .map_err(|_| failure("Datasource row decode failed."))?;
    if raw.is_null() {
        return Ok(Value::Null);
    }
    let integer_message = "Datasource integer decode failed.";
    let number_message = "Datasource number decode failed.";
    match raw.type_info().name() {
        "BOOLEAN" => {
            let value: bool = decode_column(row, index, "Datasource boolean decode failed.")?;
            Ok(json!(value))
        }
        "SMALLINT" => Ok(json!(decode_column::<i16>(row, index, integer_message)?)),
        "INTEGER" => Ok(json!(decode_column::<i32>(row, index, integer_message)?)),
        "BIGINT" => Ok(json!(decode_column::<i64>(row, index, integer_message)?)),
        "REAL" => {
            let value: f32 = decode_column(row, index, number_message)?;
            finite_number(f64::from(value))
        }
        "DOUBLE" => finite_number(decode_column(row, index, number_message)?),
        "TEXT" => {
            let value: String = decode_column(row, index, "Datasource text decode failed.")?;
            Ok(json!(value))
        }
        "BLOB" => {
            let value: Vec<u8> = decode_column(row, index, "Datasource binary decode failed.")?;
            Ok(json!(value))
        }
        _ => Err(failure(
            "SQL column type is not JSON-compatible; cast it explicitly in the query.",
        )),
    }
}

fn decode_column<'row, T>(
    row: &'row sqlx::any::AnyRow,
    index: usize,
    message: &'static str,
) -> Result<T, Failure>
where
    T: sqlx::Decode<'row, sqlx::Any> + sqlx::Type<sqlx::Any>,
{
    row.try_get::<T, _>(index).map_err(|_| failure(message))
}

#[cfg(feature = "storage-sqlserver")]
async fn execute_tds(client: &mut TdsClient, statement: &Statement) -> Result<Value, Failure> {
    let mut query = tiberius::Query::new(&statement.sql);
    for parameter in &statement.parameters {
        match parameter {
            Value::Null => query.bind(Option::<String>::None),
            Value::Bool(value) => query.bind(*value),
            Value::Number(value) if value.is_i64() => query.bind(value.as_i64()),
            Value::Number(value) => query.bind(value.as_f64()),
            Value::String(value) => query.bind(value.clone()),
            _ => return Err(failure("Invalid SQL Server parameter.")),
        }
    }
    if !statement.result.returns_rows() {
        let result = query
            .execute(client)
            .await
            .map_err(|_| failure("SQL Server mutation failed; no retry was performed."))?;
        return Ok(mutation_result(&statement.result, result.total()));
    }
    let mut stream = query
        .query(client)
        .await
        .map_err(|_| failure("SQL Server query failed."))?;
    let mut rows = Vec::new();
    let mut bytes = 0;
    while let Some(item) = stream
        .try_next()
        .await
        .map_err(|_| failure("SQL Server result failed."))?
    {
        if let tiberius::QueryItem::Row(row) = item {
            push_row(&mut rows, tds_row(&row)?, &mut bytes)?;
        }
    }
    rows_result(&statement.result, rows)
}

#[cfg(feature = "storage-sqlserver")]
fn tds_row(row: &tiberius::Row) -> Result<Value, Failure> {
    let mut object = Map::new();
    if row.columns().len() > 64 {
        return Err(failure("Storage row column limit exceeded."));
    }
    for (column, data) in row.cells() {
        if object.contains_key(column.name()) {
            return Err(failure("SQL result column names must be unique."));
        }
        let value = match data {
            tiberius::ColumnData::U8(value) => json!(value),
            tiberius::ColumnData::I16(value) => json!(value),
            tiberius::ColumnData::I32(value) => json!(value),
            tiberius::ColumnData::I64(value) => json!(value),
            tiberius::ColumnData::F32(value) => optional_finite_number(value.map(f64::from))?,
            tiberius::ColumnData::F64(value) => optional_finite_number(*value)?,
            tiberius::ColumnData::Bit(value) => json!(value),
            tiberius::ColumnData::String(value) => json!(value),
            tiberius::ColumnData::Binary(value) => json!(value),
            _ => {
                return Err(failure(
                    "SQL Server column type is not JSON-compatible; cast it explicitly in the query.",
                ));
            }
        };
        object.insert(column.name().into(), value);
    }
    Ok(Value::Object(object))
}

pub(super) fn push_row(
    rows: &mut Vec<Value>,
    row: Value,
    bytes: &mut usize,
) -> Result<(), Failure> {
    *bytes += serde_json::to_vec(&row)
        .map_err(|_| failure("Storage row serialization failed."))?
        .len();
    if rows.len() >= ManagedStorage::MAX_ROWS || *bytes > ManagedStorage::MAX_BYTES {
        return Err(failure(
            "Storage result exceeds row or byte limit; paginate the query.",
        ));
    }
    rows.push(row);
    Ok(())
}

#[cfg(feature = "storage-sqlserver")]
fn optional_finite_number(value: Option<f64>) -> Result<Value, Failure> {
    let Some(value) = value else {
        return Ok(Value::Null);
    };
    finite_number(value)
}

pub(super) fn finite_number(value: f64) -> Result<Value, Failure> {
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| failure("Non-finite database numbers cannot be returned as JSON."))
}

pub(super) fn rows_result(kind: &ResultKind, rows: Vec<Value>) -> Result<Value, Failure> {
    if !matches!(kind, ResultKind::Count | ResultKind::Exists) {
        return Ok(Value::Array(rows));
    }
    let count = rows
        .first()
        .and_then(|row| row["tachyon_count"].as_u64())
        .filter(|count| *count <= 9_007_199_254_740_991)
        .ok_or_else(|| failure("Invalid repository count result."))?;
    if matches!(kind, ResultKind::Exists) {
        return Ok(json!(count > 0));
    }
    Ok(json!(count))
}

pub(super) fn mutation_result(kind: &ResultKind, affected: u64) -> Value {
    match kind {
        ResultKind::Saved(value) => value.clone(),
        ResultKind::Changed => json!(affected > 0),
        ResultKind::Deleted => json!(affected),
        _ => json!({"affectedRows": affected}),
    }
}

pub(super) struct RedisStore {
    connection: Mutex<Option<ConnectionManager>>,
}

impl RedisStore {
    pub(super) async fn connect(settings: &Value) -> Result<Self, Failure> {
        object_keys(settings, &["url", "username", "password"])?;
        let raw = text(settings, "url")?;
        let scheme = if raw.starts_with("rediss:") {
            "rediss"
        } else {
            "redis"
        };
        let url = connection_url(settings, scheme)?;
        let client = redis::Client::open(url.as_str())
            .map_err(|_| failure("Invalid Redis connection configuration."))?;
        let connection = ConnectionManager::new(client)
            .await
            .map_err(|_| failure("Redis connection failed."))?;
        Ok(Self {
            connection: Mutex::new(Some(connection)),
        })
    }

    pub(super) async fn execute(&self, command: &Value) -> Result<Value, Failure> {
        object_keys(
            command,
            &["operation", "command", "key", "value", "ttlSeconds"],
        )?;
        let key = text(command, "key")?;
        if key.len() > 4096 {
            return Err(failure("Redis key is too large."));
        }
        let mut connection = self
            .connection
            .lock()
            .await
            .as_ref()
            .cloned()
            .ok_or_else(|| failure("Redis resource is closed."))?;
        match text(command, "command")? {
            "get" => redis_get(&mut connection, key).await,
            "set" => redis_set(&mut connection, key, command).await,
            "delete" => redis_delete(&mut connection, key).await,
            _ => Err(failure("Unsupported Redis operation.")),
        }
    }

    pub(super) async fn close(&self) {
        self.connection.lock().await.take();
    }
}

async fn redis_get(connection: &mut ConnectionManager, key: &str) -> Result<Value, Failure> {
    let value: Vec<u8> = redis::cmd("GETRANGE")
        .arg(key)
        .arg(0)
        .arg(ManagedStorage::MAX_BYTES)
        .query_async(connection)
        .await
        .map_err(|_| failure("Redis read failed."))?;
    if value.is_empty() {
        return Ok(Value::Null);
    }
    if value.len() > ManagedStorage::MAX_BYTES {
        return Err(failure("Redis value exceeds byte limit."));
    }
    serde_json::from_slice(&value).map_err(|_| failure("Redis value is not valid JSON."))
}

async fn redis_set(
    connection: &mut ConnectionManager,
    key: &str,
    command: &Value,
) -> Result<Value, Failure> {
    let value = command
        .get("value")
        .ok_or_else(|| failure("Redis set requires a JSON value."))?;
    let bytes = serde_json::to_vec(value).map_err(|_| failure("Redis JSON encoding failed."))?;
    let ttl = command["ttlSeconds"]
        .as_u64()
        .filter(|ttl| *ttl > 0 && *ttl <= 86400)
        .ok_or_else(|| failure("Redis set requires a TTL between 1 and 86400 seconds."))?;
    redis::cmd("SET")
        .arg(key)
        .arg(bytes)
        .arg("EX")
        .arg(ttl)
        .query_async::<()>(connection)
        .await
        .map_err(|_| failure("Redis write failed; no retry was performed."))?;
    Ok(Value::Bool(true))
}

async fn redis_delete(connection: &mut ConnectionManager, key: &str) -> Result<Value, Failure> {
    let affected: u64 = redis::cmd("DEL")
        .arg(key)
        .query_async(connection)
        .await
        .map_err(|_| failure("Redis delete failed."))?;
    Ok(json!(affected > 0))
}
