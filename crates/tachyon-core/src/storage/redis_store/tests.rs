#![allow(clippy::expect_used)]

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use super::RedisStore;

/// An in-process RESP2 server with a few scripted keys: `wrongtype` answers an error, `array`
/// answers a reply type Tachyon never expects, `slow` answers after the response timeout, and
/// deleting `drop` closes the connection without an answer.
#[derive(Clone, Default)]
struct FakeRedis {
    log: Arc<Mutex<Vec<String>>>,
    values: Arc<Mutex<HashMap<Vec<u8>, Vec<u8>>>>,
    connections: Arc<AtomicUsize>,
}

impl FakeRedis {
    async fn start() -> (Self, u16) {
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let port = listener.local_addr().expect("address").port();
        let server = Self::default();
        let accepting = server.clone();
        tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                accepting.connections.fetch_add(1, Ordering::SeqCst);
                tokio::spawn(accepting.clone().serve(stream));
            }
        });
        (server, port)
    }

    fn log(&self) -> Vec<String> {
        self.log.lock().expect("log").clone()
    }

    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    async fn serve(self, stream: TcpStream) {
        let mut stream = BufReader::new(stream);
        while let Some(command) = Self::command(&mut stream).await {
            let words: Vec<String> = command
                .iter()
                .map(|word| String::from_utf8_lossy(word).into_owned())
                .collect();
            self.log.lock().expect("log").push(words.join(" "));
            let Some(reply) = self.answer(&command).await else {
                return;
            };
            stream.write_all(&reply).await.expect("reply");
        }
    }

    async fn answer(&self, command: &[Vec<u8>]) -> Option<Vec<u8>> {
        let key = command.get(1).cloned().unwrap_or_default();
        match (command[0].as_slice(), key.as_slice()) {
            (b"DEL", b"drop") => None,
            (b"GETRANGE", b"wrongtype") => Some(b"-WRONGTYPE wrong kind of value\r\n".to_vec()),
            (b"GETRANGE", b"array") => Some(b"*0\r\n".to_vec()),
            (b"GETRANGE", b"slow") => {
                tokio::time::sleep(Duration::from_millis(800)).await;
                Some(b"$6\r\n\"late\"\r\n".to_vec())
            }
            (b"GETRANGE", _) => {
                let value = self.values.lock().expect("values").get(&key).cloned();
                let value = value.unwrap_or_default();
                Some(
                    [
                        format!("${}\r\n", value.len()).into_bytes(),
                        value,
                        b"\r\n".to_vec(),
                    ]
                    .concat(),
                )
            }
            (b"SET", _) => {
                self.values
                    .lock()
                    .expect("values")
                    .insert(key, command[2].clone());
                Some(b"+OK\r\n".to_vec())
            }
            (b"DEL", _) => {
                let removed = self.values.lock().expect("values").remove(&key).is_some();
                Some(format!(":{}\r\n", u8::from(removed)).into_bytes())
            }
            _ => Some(b"+OK\r\n".to_vec()),
        }
    }

    async fn command(stream: &mut BufReader<TcpStream>) -> Option<Vec<Vec<u8>>> {
        let count = Self::header(stream, b'*').await?;
        let mut command = Vec::new();
        for _ in 0..count {
            let length = Self::header(stream, b'$').await?;
            let mut word = vec![0; length + 2];
            stream.read_exact(&mut word).await.ok()?;
            word.truncate(length);
            command.push(word);
        }
        Some(command)
    }

    async fn header(stream: &mut BufReader<TcpStream>, kind: u8) -> Option<usize> {
        let mut line = String::new();
        stream.read_line(&mut line).await.ok()?;
        line.strip_prefix(char::from(kind))?.trim_end().parse().ok()
    }
}

async fn connect(settings: Value) -> RedisStore {
    RedisStore::connect(&settings)
        .await
        .unwrap_or_else(|error| panic!("{error}"))
}

async fn run(store: &RedisStore, command: &str, key: &str) -> Result<Value, String> {
    let command = json!({"operation":"redis","command":command,"key":key,"value":{"works":true},"ttlSeconds":30});
    store.execute(&command).await.map_err(message)
}

/// The diagnostic's message, without its severity and code.
fn message(error: crate::Failure) -> String {
    let text = error.to_string();
    let (_, message) = text.split_once("]: ").expect("diagnostic");
    message.lines().next().unwrap_or_default().to_owned()
}

#[tokio::test]
async fn authenticates_selects_and_speaks_resp2() {
    let (server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}/2"),"username":"app","password":"p@ss word"})).await;
    assert_eq!(run(&store, "set", "cart").await, Ok(json!(true)));
    assert_eq!(run(&store, "get", "cart").await, Ok(json!({"works":true})));
    assert_eq!(run(&store, "get", "missing").await, Ok(Value::Null));
    assert_eq!(run(&store, "delete", "cart").await, Ok(json!(true)));
    assert_eq!(run(&store, "delete", "cart").await, Ok(json!(false)));
    assert_eq!(
        server.log(),
        [
            "AUTH app p@ss word",
            "SELECT 2",
            "SET cart {\"works\":true} EX 30",
            "GETRANGE cart 0 1048576",
            "GETRANGE missing 0 1048576",
            "DEL cart",
            "DEL cart",
        ]
    );
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn sends_auth_only_with_a_password_and_select_only_off_database_zero() {
    let (server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}"),"username":"app"})).await;
    run(&store, "get", "key").await.expect("get");
    let (with_password, port) = FakeRedis::start().await;
    let other =
        connect(json!({"url":format!("redis://127.0.0.1:{port}/0"),"password":"secret"})).await;
    run(&other, "get", "key").await.expect("get");
    assert_eq!(server.log(), ["GETRANGE key 0 1048576"]);
    assert_eq!(
        with_password.log(),
        ["AUTH secret", "GETRANGE key 0 1048576"]
    );
}

#[tokio::test]
async fn an_error_reply_fails_the_command_and_keeps_the_connection() {
    let (server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}")})).await;
    assert_eq!(
        run(&store, "get", "wrongtype").await,
        Err("Redis read failed.".to_owned())
    );
    assert_eq!(run(&store, "get", "key").await, Ok(Value::Null));
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn a_late_reply_times_out_without_reaching_the_next_command() {
    let (server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}")})).await;
    run(&store, "set", "key").await.expect("set");
    assert_eq!(
        run(&store, "get", "slow").await,
        Err("Redis read failed.".to_owned())
    );
    assert_eq!(run(&store, "get", "key").await, Ok(json!({"works":true})));
    assert_eq!(server.connections(), 1);
}

#[tokio::test]
async fn a_dropped_connection_fails_the_command_in_flight_and_the_next_one_reconnects() {
    let (server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}")})).await;
    assert_eq!(
        run(&store, "delete", "drop").await,
        Err("Redis delete failed.".to_owned())
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(run(&store, "get", "key").await, Ok(Value::Null));
    assert_eq!(server.connections(), 2);
    assert_eq!(
        server
            .log()
            .iter()
            .filter(|line| *line == "DEL drop")
            .count(),
        1
    );
}

#[tokio::test]
async fn an_unexpected_reply_type_closes_the_connection() {
    let (server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}")})).await;
    assert_eq!(
        run(&store, "get", "array").await,
        Err("Redis read failed.".to_owned())
    );
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(run(&store, "get", "key").await, Ok(Value::Null));
    assert_eq!(server.connections(), 2);
}

#[tokio::test]
async fn a_closed_store_refuses_commands() {
    let (_server, port) = FakeRedis::start().await;
    let store = connect(json!({"url":format!("redis://127.0.0.1:{port}")})).await;
    store.close().await;
    assert_eq!(
        run(&store, "get", "key").await,
        Err("Redis resource is closed.".to_owned())
    );
}

#[tokio::test]
async fn rediss_does_not_fall_back_to_plain_text() {
    let (server, port) = FakeRedis::start().await;
    let refused = RedisStore::connect(&json!({"url":format!("rediss://127.0.0.1:{port}")})).await;
    assert_eq!(
        refused.err().map(message),
        Some("Redis connection failed.".to_owned())
    );
    assert!(server.log().is_empty());
}

#[tokio::test]
async fn an_unreachable_redis_fails_at_startup() {
    let port = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind")
        .local_addr()
        .expect("address")
        .port();
    let refused = RedisStore::connect(&json!({"url":format!("redis://127.0.0.1:{port}")})).await;
    assert_eq!(
        refused.err().map(message),
        Some("Redis connection failed.".to_owned())
    );
}

#[tokio::test]
async fn a_database_that_is_not_a_number_is_a_configuration_error() {
    let refused = RedisStore::connect(&json!({"url":"redis://127.0.0.1:6379/cache"})).await;
    assert_eq!(
        refused.err().map(message),
        Some("Invalid Redis connection configuration.".to_owned())
    );
}
