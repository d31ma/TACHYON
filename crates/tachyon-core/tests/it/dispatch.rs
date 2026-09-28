#![allow(clippy::expect_used)]

use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

fn write(path: &Path, contents: &str) {
    fs::create_dir_all(path.parent().expect("parent")).expect("directory");
    fs::write(path, contents).expect("source");
}

fn write_project(root: &Path) {
    write(&root.join("client/pages/tac.html"), "<test-view1 />");
    write(
        &root.join("client/components/test/view1/tac.html"),
        "<main aria-label=\"Home\"><h1>Home</h1></main>",
    );
    write(
        &root.join("server/controllers/ItemsController.js"),
        r"
@RequestRoute('/items/{id}')
@Public
export class ItemsController {
  @GetRoute
  get(@FromPath id, request) {
    return { id, method: request.method, requestId: request.headers.get('x-tachyon-request-id') }
  }
  @PostRoute
  async post(@FromPath id, request) {
    return { created: id, body: await request.text() }
  }
}
",
    );
}

async fn request(port: u16, raw: &str) -> String {
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    stream.write_all(raw.as_bytes()).await.expect("write");
    stream.flush().await.expect("flush");
    let mut response = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), stream.read_to_end(&mut response))
        .await
        .expect("response within the deadline")
        .expect("read");
    String::from_utf8_lossy(&response).into_owned()
}

async fn get(port: u16, path: &str) -> String {
    request(
        port,
        &format!("GET {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n"),
    )
    .await
}

fn typed_items_source() -> &'static str {
    r"/**
 * @typedef {Object} Item
 * @property {string} name
 * @property {string} [note]
 */
@RequestRoute('/items/{id}')
@Public
export class ItemsController {
  /** @param {number} id */
  @GetRoute
  get(@FromPath id) { return { id }; }

  /**
   * @param {number} id
   * @param {number} authorization
   * @param {Item} item
   */
  @PostRoute
  post(@FromPath id, @FromHeader authorization, @FromBody item) {
    return { created: id, body: item };
  }
}
"
}

#[tokio::test]
async fn models_enforce_captured_request_contracts_before_handler_dispatch() {
    let root = tempfile::tempdir().expect("project");
    write_project(root.path());
    let declaration = root.path().join("server/controllers/ItemsController.js");
    write(&declaration, typed_items_source());
    let captured = tachyon_core::ProjectDiscovery::discover(root.path()).expect("captured project");
    write(
        &declaration,
        &typed_items_source().replace("{number}", "{string}"),
    );
    let server = DevServer::bind_project(
        &captured,
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("model preflight and bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(server.run_until(async {
        let _ = shutdown.await;
    }));
    assert_model_request_outcomes(port).await;
    let options = request(
        port,
        "OPTIONS /items/7 HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        options.contains("200 OK") && options.contains("Item") && options.contains("allow:"),
        "{options}"
    );
    let api = get(port, "/api.json").await;
    assert!(
        api.contains("/items/_id") && api.contains("authorization"),
        "{api}"
    );
    let viewer = get(port, "/api-docs/").await;
    assert!(
        viewer.contains("200 OK") && viewer.contains("API reference"),
        "{viewer}"
    );
    stop.send(()).expect("stop");
    running.await.expect("server task").expect("shutdown");
}

async fn assert_model_request_outcomes(port: u16) {
    for (path, body, header, expected, part) in [
        ("/items/7", r#"{"name":"Ada"}"#, "7", "200 OK", ""),
        ("/items/7", r#"{"name":7}"#, "7", "400 Bad Request", "body"),
        (
            "/items/7",
            r#"{"name":"Ada","extra":1}"#,
            "7",
            "400 Bad Request",
            "body",
        ),
        ("/items/7", "not json", "7", "400 Bad Request", "body"),
        ("/items/7", "[]", "7", "400 Bad Request", "body"),
        (
            "/items/wrong",
            r#"{"name":"Ada"}"#,
            "7",
            "400 Bad Request",
            "path",
        ),
        (
            "/items/7",
            r#"{"name":"Ada"}"#,
            "PRIVATE_SECRET_TOKEN",
            "400 Bad Request",
            "header",
        ),
    ] {
        let response = request(port, &format!("POST {path} HTTP/1.1\r\nHost: localhost\r\nAuthorization: {header}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len())).await;
        assert!(response.contains(expected), "{response}");
        if !part.is_empty() {
            assert!(
                response.contains(&format!("\"part\":\"{part}\"")),
                "{response}"
            );
            assert!(
                !response.contains("PRIVATE_SECRET_TOKEN"),
                "validator messages cannot expose headers"
            );
            assert!(
                !response.contains("created"),
                "rejected input reached handler"
            );
        }
    }
}

#[tokio::test]
async fn invalid_route_models_are_refused_before_publishing_output() {
    for model in ["MissingItem", "MissingItem[]", "?MissingItem"] {
        let root = tempfile::tempdir().expect("project");
        write_project(root.path());
        write(
            &root.path().join("server/controllers/ItemsController.js"),
            &typed_items_source().replace("{Item}", &format!("{{{model}}}")),
        );
        let failure = DevServer::bind(
            root.path(),
            &DevServerOptions {
                port: 0,
                watch: false,
                ..DevServerOptions::default()
            },
        )
        .await
        .expect_err("undefined route model");
        assert!(failure.to_string().contains("TY2015"), "{failure}");
        assert!(failure.to_string().contains("MissingItem"), "{failure}");
        assert!(
            !root.path().join("dist/web").exists(),
            "invalid contract published output"
        );
    }
}

async fn read_until(stream: &mut TcpStream, needle: &str) -> String {
    let mut received = Vec::new();
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut chunk = [0_u8; 4_096];
        loop {
            let count = stream.read(&mut chunk).await.expect("stream read");
            assert_ne!(count, 0, "stream closed before {needle}");
            received.extend_from_slice(&chunk[..count]);
            if String::from_utf8_lossy(&received).contains(needle) {
                return;
            }
        }
    })
    .await
    .expect("stream event within the deadline");
    String::from_utf8_lossy(&received).into_owned()
}

#[cfg(unix)]
async fn wait_for_stream_close(stream: &mut TcpStream) {
    tokio::time::timeout(Duration::from_secs(5), async {
        let mut bytes = [0_u8; 1_024];
        loop {
            match stream.read(&mut bytes).await {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
        }
    })
    .await
    .expect("stream closes within the server shutdown bound");
}

#[cfg(unix)]
fn process_exists(pid: &str) -> bool {
    std::process::Command::new("kill")
        .args(["-0", pid.trim()])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[tokio::test]
async fn a_generator_handler_is_served_as_server_sent_events() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view2 />",
    );
    write(
        &project.path().join("client/components/test/view2/tac.html"),
        "<main>Stream</main>",
    );
    write(
        &project.path().join("server/controllers/TicksController.py"),
        "@request_route('/ticks')\n@public\nclass TicksController:\n    @stream_route\n    \
         def stream(self, request):\n        yield from self._ticks()\n\n    \
         def _ticks(self):\n        for tick in range(1, 4):\n            \
         yield {\"tick\": tick}\n",
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        let _served = server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await;
    });

    let streamed = get(port, "/ticks").await;
    assert!(streamed.contains("text/event-stream"), "{streamed}");
    for tick in 1..=3 {
        assert!(
            streamed.contains(&format!("data: {{\"tick\":{tick}}}")),
            "{streamed}"
        );
    }

    let _stopped = stop.send(());
    let _finished = running.await;
}

#[tokio::test]
async fn a_fast_finite_stream_delivers_every_event_in_exact_order() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view3 />",
    );
    write(
        &project.path().join("client/components/test/view3/tac.html"),
        "<main>Stream</main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/FiniteController.py"),
        "@request_route('/finite')\n@public\nclass FiniteController:\n    @stream_route\n    \
         def stream(self, _request):\n        yield from self._sequences()\n\n    \
         def _sequences(self):\n        for sequence in range(64):\n            \
         yield {\"sequence\": sequence}\n",
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        let _served = server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await;
    });

    let streamed = get(port, "/finite").await;
    let events = streamed
        .lines()
        .filter_map(|line| line.strip_prefix("data: "))
        .filter(|data| data.starts_with("{\"sequence\":"))
        .collect::<Vec<_>>();
    let expected = (0..64)
        .map(|sequence| format!("{{\"sequence\":{sequence}}}"))
        .collect::<Vec<_>>();
    assert_eq!(events, expected, "finite stream lost or reordered an event");
    assert!(!streamed.contains("event: error"), "{streamed}");
    assert!(
        streamed.contains("event: end\ndata: {}\n\n"),
        "a finished stream tells the browser not to reconnect: {streamed}"
    );

    let _stopped = stop.send(());
    let _finished = running.await;
}

#[cfg(unix)]
#[tokio::test]
async fn shutdown_closes_active_handler_hot_and_topic_streams_and_reaps_the_handler() {
    let project = tempfile::tempdir().expect("project");
    let pid_path = project.path().join("stream.pid");
    let effects_path = project.path().join("stream.effects");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view4 />",
    );
    write(
        &project.path().join("client/components/test/view4/tac.html"),
        "<main>Streams</main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/InfiniteController.py"),
        &format!(
            "from pathlib import Path\nimport os\nimport time\n@request_route('/infinite')\n@public\nclass InfiniteController:\n    @stream_route\n    def stream(self, _request):\n        Path({pid_path:?}).write_text(str(os.getpid()))\n        yield from self._forever()\n\n    def _forever(self):\n        while True:\n            with Path({effects_path:?}).open('a') as effects:\n                effects.write('event\\n')\n            yield {{\"event\": \"owned\"}}\n            time.sleep(0.03)\n"
        ),
    );
    write(
        &project.path().join("data/topics/chat.jsonl"),
        "{\"message\":\"owned\"}\n",
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: true,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await
    });

    let mut handler = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("handler connect");
    handler
        .write_all(b"GET /infinite HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("handler request");
    assert!(
        read_until(&mut handler, "data: {\"event\":\"owned\"}")
            .await
            .contains("200 OK")
    );

    let mut hot = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("hot connect");
    hot.write_all(b"GET /.tachyon/hot HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("hot request");
    assert!(
        read_until(&mut hot, ": connected")
            .await
            .contains("text/event-stream")
    );

    let mut topic = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("topic connect");
    topic
        .write_all(b"GET /.tachyon/topics/chat HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("topic request");
    assert!(
        read_until(&mut topic, "data: {\"message\":\"owned\"}")
            .await
            .contains("200 OK")
    );

    let pid = fs::read_to_string(&pid_path).expect("stream handler pid");
    stop.send(()).expect("shutdown signal");
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("bounded server shutdown")
        .expect("server task")
        .expect("clean shutdown");
    wait_for_stream_close(&mut handler).await;
    wait_for_stream_close(&mut hot).await;
    wait_for_stream_close(&mut topic).await;
    let settled = fs::read_to_string(&effects_path).expect("settled stream effects");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        fs::read_to_string(&effects_path).expect("settled stream effects"),
        settled,
        "stream handler produced effects after shutdown"
    );
    assert!(
        !process_exists(&pid),
        "stream handler process {pid} survived"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn disconnecting_a_stream_client_reaps_its_handler_without_stopping_the_server() {
    let project = tempfile::tempdir().expect("project");
    let pid_path = project.path().join("disconnect.pid");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view5 />",
    );
    write(
        &project.path().join("client/components/test/view5/tac.html"),
        "<main>Disconnect</main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/InfiniteController.py"),
        &format!(
            "from pathlib import Path\nimport os\nimport time\n@request_route('/infinite')\n@public\nclass InfiniteController:\n    @stream_route\n    def stream(self, _request):\n        Path({pid_path:?}).write_text(str(os.getpid()))\n        yield from self._forever()\n\n    def _forever(self):\n        while True:\n            yield {{\"event\": \"active\"}}\n            time.sleep(0.03)\n"
        ),
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await
    });
    let mut stream = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect");
    stream
        .write_all(b"GET /infinite HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("request");
    let _first = read_until(&mut stream, "data: {\"event\":\"active\"}").await;
    let pid = fs::read_to_string(&pid_path).expect("handler pid");
    drop(stream);
    tokio::time::timeout(Duration::from_secs(3), async {
        while process_exists(&pid) {
            tokio::time::sleep(Duration::from_millis(25)).await;
        }
    })
    .await
    .expect("disconnected handler is reaped");
    assert!(get(port, "/").await.contains("200 OK"));
    stop.send(()).expect("shutdown signal");
    running.await.expect("server task").expect("clean shutdown");
}

#[tokio::test]
async fn a_php_generator_streams_through_the_direct_protocol() {
    if !available("php-cgi") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view6 />",
    );
    write(
        &project.path().join("client/components/test/view6/tac.html"),
        "<main>Stream</main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/TicksController.php"),
        "<?php\n#[RequestRoute('/ticks')]\n#[Public]\nclass TicksController\n{\n    #[StreamRoute]\n    \
         public function stream()\n    {\n        \
         yield from $this->ticks([1, 2, 3]);\n    }\n\n    \
         private function ticks(array $values)\n    {\n        \
         foreach ($values as $tick) {\n            yield ['tick' => $tick];\n        \
         }\n    }\n}\n",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        let _served = server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await;
    });

    let streamed = get(port, "/ticks").await;
    assert!(streamed.contains("text/event-stream"), "{streamed}");
    for tick in 1..=3 {
        assert!(
            streamed.contains(&format!("data: {{\"tick\":{tick}}}")),
            "{streamed}"
        );
    }

    let _stopped = stop.send(());
    let _finished = running.await;
}

#[tokio::test]
async fn a_stream_failure_does_not_expose_handler_error_text() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view7 />",
    );
    write(
        &project.path().join("client/components/test/view7/tac.html"),
        "<main>Stream</main>",
    );
    write(
        &project.path().join("server/controllers/TicksController.py"),
        "@request_route('/ticks')\n@public\nclass TicksController:\n    @stream_route\n    \
         def stream(self, request):\n        yield {\"tick\": 1}\n        \
         raise RuntimeError(\"secret-canary\")\n",
    );
    write(
        &project.path().join("server/controllers/FailController.py"),
        "@request_route('/fail')\n@public\nclass FailController:\n    @stream_route\n    def stream(self, request):\n        raise RuntimeError(\"before-first-secret\")\n        yield None\n",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        let _served = server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await;
    });

    let streamed = get(port, "/ticks").await;
    assert!(streamed.contains("data: {\"tick\":1}"), "{streamed}");
    assert!(streamed.contains("event: error"), "{streamed}");
    assert!(
        streamed.find("data: {\"tick\":1}") < streamed.find("event: error"),
        "data must precede the terminal error: {streamed}"
    );
    assert_eq!(streamed.matches("event: error").count(), 1, "{streamed}");
    assert!(streamed.contains("\"code\":\"TY2107\""), "{streamed}");
    let request_id = streamed
        .lines()
        .find_map(|line| {
            const HEADER: &str = "x-tachyon-request-id: ";
            line.get(..HEADER.len())
                .is_some_and(|name| name.eq_ignore_ascii_case(HEADER))
                .then(|| String::from(&line[HEADER.len()..]))
        })
        .expect("request id header");
    assert!(
        streamed.contains(&format!("\"request_id\":\"{}\"", request_id.trim())),
        "{streamed}"
    );
    assert!(!streamed.contains("secret-canary"), "{streamed}");

    let before_first = get(port, "/fail").await;
    assert!(before_first.contains("event: error"), "{before_first}");
    assert_eq!(
        before_first.matches("event: error").count(),
        1,
        "{before_first}"
    );
    assert!(
        before_first.contains("\"code\":\"TY2107\""),
        "{before_first}"
    );
    assert!(!before_first.contains("data: null"), "{before_first}");
    assert!(
        !before_first.contains("before-first-secret"),
        "{before_first}"
    );
    let before_request_id = before_first
        .lines()
        .find_map(|line| {
            const HEADER: &str = "x-tachyon-request-id: ";
            line.get(..HEADER.len())
                .is_some_and(|name| name.eq_ignore_ascii_case(HEADER))
                .then(|| String::from(&line[HEADER.len()..]))
        })
        .expect("request id header");
    assert!(
        before_first.contains(&format!("\"request_id\":\"{}\"", before_request_id.trim())),
        "{before_first}"
    );

    let _stopped = stop.send(());
    let _finished = running.await;
}

#[tokio::test]
async fn requests_reach_supervised_handlers_with_bound_parameters() {
    let project = tempfile::tempdir().expect("project");
    write_project(project.path());

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let dynamic = get(port, "/items/42").await;
    assert!(dynamic.contains("200 OK"), "{dynamic}");
    assert!(dynamic.contains(r#""id":"42""#), "{dynamic}");
    assert!(dynamic.contains(r#""method":"GET""#), "{dynamic}");

    let id_start = dynamic.find(r#""requestId":""#).expect("request id");
    let request_id: String = dynamic[id_start + 13..]
        .chars()
        .take_while(|character| *character != '"')
        .collect();
    assert_eq!(request_id.len(), 11, "{request_id}");
    assert!(
        request_id
            .bytes()
            .all(|byte| byte.is_ascii_uppercase() || byte.is_ascii_digit()),
        "{request_id}"
    );

    let encoded = get(port, "/items/a%20b").await;
    assert!(encoded.contains(r#""id":"a b""#), "{encoded}");

    let posted = request(
        port,
        "POST /items/7 HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 7\r\n\
         Content-Type: application/json\r\nConnection: close\r\n\r\n{\"x\":1}",
    )
    .await;
    assert!(posted.contains(r#""created":"7""#), "{posted}");

    let refused = request(
        port,
        "DELETE /items/7 HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(
        refused.contains("405") || refused.contains("404"),
        "{refused}"
    );

    let index = get(port, "/").await;
    assert!(
        index.contains("200 OK") && index.contains("Home"),
        "{index}"
    );
    let missing = get(port, "/nothing-here").await;
    assert!(missing.contains("404"), "{missing}");

    let traversal = get(port, "/items/%2e%2e%2fsecret").await;
    assert!(
        traversal.contains("404") || traversal.contains("400"),
        "{traversal}"
    );

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn explicit_html_handler_responses_pass_through_without_rendering() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view8 />",
    );
    write(
        &project.path().join("client/components/test/view8/tac.html"),
        "<main>Client page</main>",
    );
    write(
        &project.path().join("server/controllers/HtmlController.js"),
        r"
@RequestRoute('/html')
@Public
export class HtmlController {
  @GetRoute
  get() {
    return new Response('<article><h1>Unchanged HTML</h1><p>{not-a-template}</p></article>', {
      status: 202,
      headers: { 'Content-Type': 'text/html; charset=utf-8', 'X-Origin': 'handler' },
    })
  }
}",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let response = get(port, "/html").await;
    assert!(response.contains("202 Accepted"), "{response}");
    assert!(
        response.contains("content-type: text/html; charset=utf-8"),
        "{response}"
    );
    assert!(response.contains("x-origin: handler"), "{response}");
    assert!(
        response.contains("<article><h1>Unchanged HTML</h1><p>{not-a-template}</p></article>"),
        "{response}"
    );
    assert!(!response.contains("tachyon-view"), "{response}");

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn a_project_without_handlers_still_serves_generated_output() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<page-only />",
    );
    write(
        &project.path().join("client/components/page/only/tac.html"),
        "<main aria-label=\"Only\"><h1>Only</h1></main>",
    );
    write(
        &project.path().join("client/pages/docs/tac.html"),
        "<docs-index hydrate=\"load\" />",
    );
    write(
        &project.path().join("client/components/docs/index/tac.html"),
        "<main><h1>Documentation index</h1><p>{documentsIndex.marker}</p></main>",
    );
    write(
        &project
            .path()
            .join("client/services/DocumentsIndexService.js"),
        "@Service()\nexport default class DocumentsIndexService { marker = 'STATIC_CLIENT_ASSET' }\n",
    );
    write(
        &project.path().join("client/pages/docs/_topic/tac.html"),
        "<docs-topic />",
    );
    write(
        &project.path().join("client/components/docs/topic/tac.html"),
        "<main><h1>Dynamic documentation template</h1></main>",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let index = get(port, "/").await;
    assert!(
        index.contains("200 OK") && index.contains("Only"),
        "{index}"
    );
    let dynamic = get(port, "/docs/routing").await;
    assert!(
        dynamic.contains("200 OK") && dynamic.contains("Dynamic documentation template"),
        "{dynamic}"
    );
    let client_asset = get(port, "/.tachyon/services/DocumentsIndexService.js").await;
    assert!(client_asset.contains("200 OK"), "{client_asset}");
    assert!(
        client_asset.contains("STATIC_CLIENT_ASSET"),
        "a generated asset lost precedence over a dynamic page: {client_asset}"
    );
    assert!(
        !client_asset.contains("Dynamic documentation template"),
        "{client_asset}"
    );

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn watching_rebuilds_and_survives_a_broken_source() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<app-view />",
    );
    let source = project.path().join("client/components/app/view/tac.html");
    write(&source, "<main aria-label=\"W\"><h1>First</h1></main>");

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: true,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let output = server
        .build()
        .expect("initial build")
        .output_directory()
        .join("index.html");
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    assert_hot_update_client(port).await;

    let mut updates = TcpStream::connect(("127.0.0.1", port))
        .await
        .expect("connect hot stream");
    updates
        .write_all(
            b"GET /.tachyon/hot HTTP/1.1\r\nHost: 127.0.0.1\r\nAccept: text/event-stream\r\n\r\n",
        )
        .await
        .expect("open hot stream");
    let connected = read_until(&mut updates, ": connected").await;
    assert!(
        connected.contains("content-type: text/event-stream"),
        "{connected}"
    );
    assert!(connected.contains("x-accel-buffering: no"), "{connected}");

    assert_watched_source_recovery(port, &source, &mut updates).await;

    drop(updates);
    let _stopped = stop.send(());
    running
        .await
        .expect("server task")
        .expect("graceful shutdown");

    assert_watcher_stopped(&output, &source).await;
}

async fn assert_watcher_stopped(output: &Path, source: &Path) {
    let settled = fs::read(output).expect("settled output");
    write(
        source,
        "<main aria-label=\"W\"><h1>After shutdown</h1></main>",
    );
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert_eq!(
        fs::read(output).expect("post-shutdown output"),
        settled,
        "watcher rebuilt after public server shutdown"
    );
}

#[tokio::test]
async fn no_watch_serves_documents_untouched() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view9 />",
    );
    write(
        &project.path().join("client/components/test/view9/tac.html"),
        "<main aria-label=\"P\"><h1>Plain</h1></main>",
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let document = get(port, "/").await;
    assert!(document.contains("Plain"), "{document}");
    assert!(!document.contains("live-reload"), "reload client leaked in");
    for endpoint in [
        "/.tachyon/hot",
        "/.tachyon/live",
        "/.tachyon/live-reload.js",
        "/.tachyon/diagnostics",
    ] {
        let response = get(port, endpoint).await;
        assert!(response.contains("404 Not Found"), "{endpoint}: {response}");
    }

    let _stopped = stop.send(());
    let _joined = running.await;
}

fn available(program: &str) -> bool {
    std::process::Command::new(program)
        .arg("--version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[tokio::test]
async fn unsupported_route_languages_are_reached_through_relay() {
    if !available("ruby") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view10 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view10/tac.html"),
        "<main aria-label=\"P\"><h1>Polyglot</h1></main>",
    );

    write(
        &project.path().join("server/controllers/GreetController.py"),
        "@request_route('/greet/{name}')\n@public\nclass GreetController:\n    @get_route\n    def get(self, request):\n        \
         return relay((\"ruby\", \"server/helpers/greet.rb\"), request)\n",
    );
    write(
        &project.path().join("server/helpers/greet.rb"),
        "require 'json'\n\
         request = JSON.parse($stdin.read)\n\
         name = request['route'].split('/').last\n\
         puts JSON.generate({ status: 200, body: JSON.generate({ hello: name }) })\n",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let greeted = get(port, "/greet/Ada").await;
    assert!(greeted.contains(r#"{"hello":"Ada"}"#), "{greeted}");

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn an_unrunnable_handler_extension_fails_closed() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view11 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view11/tac.html"),
        "<main aria-label=\"P\"><h1>P</h1></main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/ThingController.zig"),
        "// no interpreter registered, not executable\n",
    );

    let error = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect_err("unrunnable handler");
    let text = error.to_string();
    assert!(text.contains("TY2003"), "{text}");
    assert!(text.contains("ThingController.zig"), "{text}");
    assert!(text.contains("relay("), "{text}");
}

#[tokio::test]
async fn middleware_can_refuse_a_request_before_it_reaches_a_route() {
    if !available("php-cgi") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view12 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view12/tac.html"),
        "<main aria-label=\"M\"><h1>Open</h1></main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/PrivateController.py"),
        "@request_route('/private')\n@public\nclass PrivateController:\n    @get_route\n    def get(self, request):\n        \
         return (200, {}, \"secret data\")\n",
    );
    write(
        &project.path().join("middleware.php"),
        r"<?php
class Middleware {
    public static function before(): void {
        $token = $_SERVER['HTTP_AUTHORIZATION'] ?? '';
        if (str_starts_with($_SERVER['REQUEST_URI'], '/private') && $token !== 'let-me-in') {
            http_response_code(401);
            header('Content-Type: application/json');
            echo 'unauthorized';
        }
    }
}
",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let refused = get(port, "/private").await;
    assert!(refused.contains("401"), "{refused}");
    assert!(!refused.contains("secret data"), "{refused}");

    let allowed = request(
        port,
        "GET /private HTTP/1.1\r\nHost: 127.0.0.1\r\nAuthorization: let-me-in\r\n\
         Connection: close\r\n\r\n",
    )
    .await;
    assert!(allowed.contains("secret data"), "{allowed}");

    let open = get(port, "/").await;
    assert!(open.contains("Open"), "{open}");

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn middleware_can_adjust_a_response_after_it_is_produced() {
    if !available("php-cgi") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view13 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view13/tac.html"),
        "<main aria-label=\"A\"><h1>After</h1></main>",
    );
    write(
        &project.path().join("server/controllers/ApiController.py"),
        "@request_route('/api')\n@public\nclass ApiController:\n    @get_route\n    def get(self, request):\n        \
         return (200, {}, '{\"ok\":true}')\n",
    );
    write(
        &project.path().join("middleware.php"),
        r"<?php
class Middleware {
    public static function after(): void {
        $status = $_SERVER['HTTP_X_TACHYON_STATUS'] ?? '';
        header('x-served-by: tachyon');
        header('x-observed-status: ' . $status);
    }
}
",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let handled = get(port, "/api").await;
    assert!(handled.contains("x-served-by: tachyon"), "{handled}");
    assert!(handled.contains("x-observed-status: 200"), "{handled}");
    assert!(handled.contains(r#"{"ok":true}"#), "{handled}");

    let document = get(port, "/").await;
    assert!(document.contains("x-served-by: tachyon"), "{document}");
    assert!(document.contains("After"), "{document}");

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn php_middleware_passes_options_and_form_posts_through() {
    if !available("php-cgi") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view21 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view21/tac.html"),
        "<main aria-label=\"Form\"><h1>Form</h1></main>",
    );
    write(
        &project.path().join("server/controllers/FormController.py"),
        "@request_route('/form')\n@public\nclass FormController:\n    @post_route\n    def post(self, request):\n        \
         return (201, {}, \"post-route\")\n",
    );
    write(
        &project.path().join("middleware.php"),
        r"<?php
final class Middleware
{
    public static function before(): void
    {
    }

    public static function after(): void
    {
        header('x-middleware-after: ' . $_SERVER['REQUEST_METHOD']);
    }
}
",
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let options = request(
        port,
        "OPTIONS /form HTTP/1.1\r\nHost: 127.0.0.1\r\nOrigin: http://example.test\r\n\
         Access-Control-Request-Method: POST\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(options.starts_with("HTTP/1.1 200"), "{options}");
    assert!(options.contains("allow: OPTIONS, POST"), "{options}");
    assert!(options.contains("x-middleware-after: OPTIONS"), "{options}");

    let form = "name=Ada";
    let posted = request(
        port,
        &format!(
            "POST /form HTTP/1.1\r\nHost: 127.0.0.1\r\n\
             Content-Type: application/x-www-form-urlencoded\r\nContent-Length: {}\r\n\
             Connection: close\r\n\r\n{form}",
            form.len()
        ),
    )
    .await;
    assert!(posted.starts_with("HTTP/1.1 201"), "{posted}");
    assert!(posted.contains("post-route"), "{posted}");
    assert!(posted.contains("x-middleware-after: POST"), "{posted}");

    let _stopped = stop.send(());
    let _joined = running.await;
}

fn write_middleware_project(root: &Path, model: (&str, &str), middleware: (&str, &str)) {
    write(&root.join("client/pages/tac.html"), "<test-view20 />");
    write(
        &root.join("client/components/test/view20/tac.html"),
        "<main aria-label=\"Models\"><h1>Models</h1></main>",
    );
    let (model_path, model_source) = model;
    write(&root.join(model_path), model_source);
    let (middleware_path, middleware_source) = middleware;
    write(&root.join(middleware_path), middleware_source);
}

async fn get_once(project: &Path, path: &str) -> String {
    let server = DevServer::bind(
        project,
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });
    let response = get(port, path).await;
    let _stopped = stop.send(());
    let _joined = running.await;
    response
}

#[tokio::test]
async fn a_python_middleware_uses_a_model() {
    if !available("python3") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write_middleware_project(
        project.path(),
        (
            "server/models/Badge.py",
            "from dataclasses import dataclass\n\n\n@dataclass\nclass Badge:\n    name: str\n    level: int\n",
        ),
        (
            "middleware.py",
            "from models import Badge\n\n\nclass Middleware:\n    @staticmethod\n    def after(request):\n        \
             badge = Badge('gold', 3)\n        \
             return (204, {'x-badge': f'{badge.name}-{badge.level}'}, '')\n",
        ),
    );

    let document = get_once(project.path(), "/").await;
    assert!(document.contains("x-badge: gold-3"), "{document}");
    assert!(document.contains("Models"), "{document}");
}

#[tokio::test]
async fn a_php_middleware_uses_a_model() {
    if !available("php-cgi") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write_middleware_project(
        project.path(),
        (
            "server/models/Badge.php",
            "<?php\nclass Badge\n{\n    public function __construct(public string $name, public int $level) {}\n}\n",
        ),
        (
            "middleware.php",
            r"<?php
use Models\Badge;

class Middleware {
    public static function after(): void {
        $badge = new Badge('silver', 2);
        header('x-badge: ' . $badge->name . '-' . $badge->level);
    }
}
",
        ),
    );

    let document = get_once(project.path(), "/").await;
    assert!(document.contains("x-badge: silver-2"), "{document}");
    assert!(document.contains("Models"), "{document}");
}

#[cfg(unix)]
async fn completed_worker_marker(path: &Path) -> String {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            match fs::read_to_string(path) {
                Ok(contents) if contents.ends_with('\n') => break contents,
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => panic!("cannot read owned worker marker: {error}"),
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("captured one-second worker schedule executed")
}

#[cfg(unix)]
#[tokio::test]
async fn startup_uses_one_owned_snapshot_for_build_middleware_and_scheduled_workers() {
    let workspace = tempfile::tempdir().expect("workspace");
    let authored = workspace.path().join("project");
    let original = workspace.path().join("original");
    let output = workspace.path().join("web-output");
    let owned_marker = workspace.path().join("owned-worker");
    let planted_marker = workspace.path().join("planted-worker");
    fs::create_dir_all(&authored).expect("project");
    write(&authored.join("client/pages/tac.html"), "<test-view14 />");
    write(
        &authored.join("client/components/test/view14/tac.html"),
        "<main><h1>Owned startup page</h1></main>",
    );
    write(
        &authored.join("middleware.py"),
        "class Middleware:\n    @staticmethod\n    def after(request):\n        return (204, {\"x-startup-snapshot\": \"owned\"}, \"\")\n",
    );
    write(
        &authored.join("server/workers/heartbeat.py"),
        &format!(
            "from pathlib import Path\nimport time\nclass Handler:\n    @staticmethod\n    def POST(request):\n        with Path({owned_marker:?}).open('a', encoding='utf-8') as marker:\n            time.sleep(0.15)\n            marker.write('owned\\n')\n        return (204, {{}}, \"\")\n"
        ),
    );
    write(
        &authored.join(".tachyonrc"),
        r#"{"workers":{"server/workers/heartbeat.py":{"every_seconds":1}}}"#,
    );

    let project = tachyon_core::ProjectDiscovery::discover(&authored).expect("owned snapshot");
    fs::rename(&authored, &original).expect("move authored root");
    fs::create_dir_all(&authored).expect("planted root");
    write(&authored.join("client/pages/tac.html"), "<test-view15 />");
    write(
        &authored.join("client/components/test/view15/tac.html"),
        "<main><h1>PLANTED startup page</h1></main>",
    );
    write(
        &authored.join("middleware.py"),
        "class Middleware:\n    @staticmethod\n    def after(request):\n        return (204, {\"x-startup-snapshot\": \"planted\"}, \"\")\n",
    );
    write(
        &authored.join("server/workers/heartbeat.py"),
        &format!(
            "from pathlib import Path\nclass Handler:\n    @staticmethod\n    def POST(request):\n        Path({planted_marker:?}).write_text('planted')\n        return (204, {{}}, \"\")\n"
        ),
    );
    write(
        &authored.join(".tachyonrc"),
        r#"{"workers":{"server/workers/heartbeat.py":{"every_seconds":86400}}}"#,
    );

    let server = DevServer::bind_project(
        &project,
        &DevServerOptions {
            port: 0,
            watch: false,
            output_directory: output,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind from snapshot");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let response = get(port, "/").await;
    assert!(response.contains("Owned startup page"), "{response}");
    assert!(response.contains("x-startup-snapshot: owned"), "{response}");
    assert!(!response.contains("PLANTED"), "{response}");
    assert!(
        !response.contains("x-startup-snapshot: planted"),
        "{response}"
    );

    let marker = completed_worker_marker(&owned_marker).await;
    assert_eq!(marker, "owned\n");
    assert!(!planted_marker.exists(), "planted worker source executed");

    let _stopped = stop.send(());
    running
        .await
        .expect("server task")
        .expect("graceful shutdown");
    let settled = fs::read_to_string(&owned_marker).expect("settled worker marker");
    tokio::time::sleep(Duration::from_millis(1_250)).await;
    assert_eq!(
        fs::read_to_string(&owned_marker).expect("post-shutdown marker"),
        settled,
        "scheduled worker continued after public server shutdown"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn dropping_a_bound_server_starts_neither_workers_nor_watcher_rebuilds() {
    if !available("python3") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<app-view />",
    );
    let source = project.path().join("client/components/app/view/tac.html");
    let marker = project.path().join("worker-marker");
    write(&source, "<main><h1>Before drop</h1></main>");
    write(
        &project.path().join("server/workers/drop.py"),
        &format!(
            "from pathlib import Path\nclass Handler:\n    @staticmethod\n    def POST(request):\n        Path({marker:?}).write_text('leaked')\n        return (204, {{}}, \"\")\n"
        ),
    );
    write(
        &project.path().join(".tachyonrc"),
        r#"{"workers":{"server/workers/drop.py":{"every_seconds":1}}}"#,
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: true,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let output = server
        .build()
        .expect("initial build")
        .output_directory()
        .join("index.html");
    let published = fs::read(&output).expect("initial output");
    drop(server);
    write(&source, "<main><h1>PLANTED after drop</h1></main>");
    tokio::time::sleep(Duration::from_millis(1_500)).await;

    assert!(
        !marker.exists(),
        "a dropped bound server started its worker"
    );
    assert_eq!(
        fs::read(&output).expect("output after drop"),
        published,
        "a dropped bound server kept a watcher rebuild alive"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn shutdown_cancels_and_reaps_a_hung_worker_within_the_server_bound() {
    if !available("python3") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    let started = project.path().join("hung-worker-started");
    let finished = project.path().join("hung-worker-finished");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view16 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view16/tac.html"),
        "<main><h1>Hung worker host</h1></main>",
    );
    write(
        &project.path().join("server/workers/hung.py"),
        &format!(
            "import os\nimport time\nfrom pathlib import Path\nclass Handler:\n    @staticmethod\n    def POST(request):\n        Path({started:?}).write_text(str(os.getpid()))\n        time.sleep(60)\n        Path({finished:?}).write_text('leaked')\n        return (204, {{}}, \"\")\n"
        ),
    );
    write(
        &project.path().join(".tachyonrc"),
        r#"{"workers":{"server/workers/hung.py":{"every_seconds":1}}}"#,
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(8), async {
        while !started.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("hung worker entered user code");
    let pid = fs::read_to_string(&started).expect("worker pid");

    let _stopped = stop.send(());
    tokio::time::timeout(Duration::from_secs(5), running)
        .await
        .expect("bounded server shutdown")
        .expect("server task")
        .expect("graceful shutdown");
    tokio::time::sleep(Duration::from_millis(300)).await;
    assert!(!finished.exists(), "hung worker completed after shutdown");
    assert!(
        !std::process::Command::new("kill")
            .args(["-0", pid.trim()])
            .status()
            .is_ok_and(|status| status.success()),
        "worker process {pid} survived server shutdown"
    );
}

#[cfg(unix)]
#[tokio::test]
async fn a_failing_worker_does_not_stop_the_http_server() {
    if !available("python3") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    let marker = project.path().join("failed-worker-ran");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view17 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view17/tac.html"),
        "<main><h1>Still serving</h1></main>",
    );
    write(
        &project.path().join("server/workers/fails.py"),
        &format!(
            "from pathlib import Path\nclass Handler:\n    @staticmethod\n    def POST(request):\n        Path({marker:?}).write_text('ran')\n        return (500, {{}}, \"failed\")\n"
        ),
    );
    write(
        &project.path().join(".tachyonrc"),
        r#"{"workers":{"server/workers/fails.py":{"every_seconds":1}}}"#,
    );
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });
    tokio::time::timeout(Duration::from_secs(8), async {
        while !marker.exists() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("failing worker ran");
    assert!(get(port, "/").await.contains("Still serving"));
    let _stopped = stop.send(());
    running
        .await
        .expect("server task")
        .expect("graceful shutdown");
}

#[tokio::test]
async fn removed_arbitrary_language_workers_fail_closed() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view18 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view18/tac.html"),
        "<main aria-label=\"W\"><h1>W</h1></main>",
    );
    write(
        &project.path().join(".tachyonrc"),
        r#"{"interpreters":{"rb":["ruby"]},"workers":{"server/workers/beat.rb":{"every_seconds":1}}}"#,
    );
    write(
        &project.path().join("server/workers/beat.rb"),
        "puts '{\"status\":200}'\n",
    );

    let failure = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect_err("Ruby is not a Yon worker language");
    let rendered = failure.to_string();
    assert!(rendered.contains("TY1502"), "{rendered}");
    assert!(rendered.contains("interpreters"), "{rendered}");
    assert!(rendered.contains("relay("), "{rendered}");
}

#[tokio::test]
async fn topics_stream_as_server_sent_events_with_a_resumable_cursor() {
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view19 />",
    );
    write(
        &project
            .path()
            .join("client/components/test/view19/tac.html"),
        "<main aria-label=\"R\"><h1>R</h1></main>",
    );
    let old_log = project.path().join(".tachyon/topics/chat.jsonl");
    write(&old_log, "{\"seq\":1}\n{\"seq\":2}\n");

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let log = project.path().join("data/topics/chat.jsonl");
    assert!(log.is_file(), "the old topic log moves on start");
    assert!(!old_log.exists());
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let read_stream = |raw: String| async move {
        let mut stream = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        stream.write_all(raw.as_bytes()).await.expect("write");
        let mut seen = Vec::new();
        let mut chunk = [0_u8; 4096];
        let deadline = tokio::time::Instant::now() + Duration::from_secs(4);
        while tokio::time::Instant::now() < deadline {
            match tokio::time::timeout(Duration::from_millis(600), stream.read(&mut chunk)).await {
                Ok(Ok(read)) if read > 0 => seen.extend_from_slice(&chunk[..read]),
                _ => break,
            }
        }
        String::from_utf8_lossy(&seen).into_owned()
    };

    let from_start = read_stream(String::from(
        "GET /.tachyon/topics/chat HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
    ))
    .await;
    assert!(from_start.contains("text/event-stream"), "{from_start}");
    assert!(from_start.contains("id: 0"), "{from_start}");
    assert!(from_start.contains(r#"data: {"seq":1}"#), "{from_start}");
    assert!(from_start.contains(r#"data: {"seq":2}"#), "{from_start}");

    fs::write(&log, "{\"seq\":1}\n{\"seq\":2}\n{\"seq\":3}\n").expect("publish");
    let resumed = read_stream(String::from(
        "GET /.tachyon/topics/chat HTTP/1.1\r\nHost: 127.0.0.1\r\n\
         Last-Event-ID: 1\r\n\r\n",
    ))
    .await;
    assert!(resumed.contains(r#"data: {"seq":3}"#), "{resumed}");
    assert!(!resumed.contains(r#"data: {"seq":1}"#), "{resumed}");

    let refused = read_stream(String::from(
        "GET /.tachyon/topics/BAD..name HTTP/1.1\r\nHost: 127.0.0.1\r\n\
         Connection: close\r\n\r\n",
    ))
    .await;
    assert!(refused.contains("400"), "{refused}");

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn a_dart_async_generator_streams_through_the_direct_protocol() {
    if !available("dart") {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    write(
        &project.path().join("client/pages/tac.html"),
        "<test-view7 />",
    );
    write(
        &project.path().join("client/components/test/view7/tac.html"),
        "<main>Stream</main>",
    );
    write(
        &project
            .path()
            .join("server/controllers/TicksController.dart"),
        "@RequestRoute('/ticks')\n@Public\nclass TicksController {\n  @StreamRoute\n  Stream<Object?> stream() async* {\n    \
         yield* _ticks([1, 2, 3]);\n  }\n\n  \
         Stream<Object?> _ticks(List<int> values) async* {\n    \
         for (final tick in values) {\n      yield {'tick': tick};\n    }\n  }\n}\n",
    );
    write(
        &project
            .path()
            .join("server/controllers/BrokenController.dart"),
        "@RequestRoute('/broken')\n@Public\nclass BrokenController {\n  @StreamRoute\n  Stream<Object?> stream() async* {\n    \
         yield {'tick': 1};\n    throw StateError('secret-canary');\n  }\n}\n",
    );
    write(
        &project
            .path()
            .join("server/controllers/BeatController.dart"),
        "@RequestRoute('/tocks')\n@Public\nclass BeatController {\n  @StreamRoute\n  Stream<Object?> stream() async* {\n    \
         yield* _beats(['a', 'b']);\n  }\n\n  \
         Stream<Object?> _beats(List<String> values) async* {\n    \
         for (final tock in values) {\n      \
         await Future<void>.delayed(const Duration(milliseconds: 5));\n      yield {'tock': tock};\n    }\n  }\n}\n",
    );

    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, shutdown) = tokio::sync::oneshot::channel();
    let running = tokio::spawn(async move {
        let _served = server
            .run_until(async {
                let _stopped = shutdown.await;
            })
            .await;
    });

    let streamed = get(port, "/ticks").await;
    assert!(streamed.contains("text/event-stream"), "{streamed}");
    for tick in 1..=3 {
        assert!(
            streamed.contains(&format!("data: {{\"tick\":{tick}}}")),
            "{streamed}"
        );
    }
    let controlled = get(port, "/tocks").await;
    assert!(controlled.contains("text/event-stream"), "{controlled}");
    assert!(
        controlled.find("data: {\"tock\":\"a\"}") < controlled.find("data: {\"tock\":\"b\"}"),
        "{controlled}"
    );
    assert!(
        controlled.contains("data: {\"tock\":\"a\"}"),
        "{controlled}"
    );
    let broken = get(port, "/broken").await;
    assert!(broken.contains("data: {\"tick\":1}"), "{broken}");
    assert_eq!(broken.matches("event: error").count(), 1, "{broken}");
    assert!(!broken.contains("secret-canary"), "{broken}");

    let _stopped = stop.send(());
    let _finished = running.await;
}

async fn assert_hot_update_client(port: u16) {
    let first = get(port, "/").await;
    assert!(first.contains("First"), "{first}");
    assert!(first.contains("/.tachyon/live-reload.js"), "{first}");
    assert!(
        !first.contains("<script>let seen"),
        "inline script was injected"
    );
    let client = get(port, "/.tachyon/live-reload.js").await;
    assert!(client.contains("new EventSource(endpoint)"), "{client}");
    assert!(client.contains("tachyon:hot-update"), "{client}");
    assert!(client.contains("updateIslands"), "{client}");
    assert!(client.contains("replaceStyles"), "{client}");
    assert!(client.contains("tac.hotUpdate"), "{client}");
    assert!(client.contains("/.tachyon/diagnostics"), "{client}");
    assert!(!client.contains("DOMParser"), "{client}");
    assert!(!client.contains("__tachyonIslands"), "{client}");
    assert!(
        !client.contains("setInterval("),
        "polling client returned: {client}"
    );
}

async fn assert_watched_source_recovery(port: u16, source: &Path, updates: &mut TcpStream) {
    let before = get(port, "/.tachyon/live").await;
    write(source, "<main aria-label=\"W\"><h1>Second</h1></main>");
    let reloaded = read_until(updates, r#""kind":"reload""#).await;
    assert!(reloaded.contains("event: hot"), "{reloaded}");
    assert!(
        reloaded.contains("client/components/app/view/tac.html"),
        "{reloaded}"
    );
    let mut rebuilt = String::new();
    for _ in 0..60 {
        tokio::time::sleep(Duration::from_millis(250)).await;
        rebuilt = get(port, "/").await;
        if rebuilt.contains("Second") {
            break;
        }
    }
    assert!(rebuilt.contains("Second"), "watch never rebuilt: {rebuilt}");
    assert_ne!(
        before,
        get(port, "/.tachyon/live").await,
        "generation stalled"
    );

    write(source, "<main><logic :else>orphan</logic></main>");
    let diagnostics = read_until(updates, r#""kind":"diagnostics""#).await;
    assert!(diagnostics.contains(r#""contract_version":1"#));
    assert!(diagnostics.contains("TY"), "{diagnostics}");
    let shown = get(port, "/.tachyon/diagnostics").await;
    assert!(shown.contains("\"code\":\"TY"), "{shown}");
    let survived = get(port, "/").await;
    assert!(
        survived.contains("Second"),
        "a failed rebuild broke the site"
    );
}

fn go_usable() -> bool {
    let found = std::process::Command::new("go")
        .arg("version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success());
    if found {
        return true;
    }
    let required = cfg!(unix) && std::env::var_os("CI").is_some();
    assert!(!required, "go is required for the Go handler tests in CI");
    eprintln!("skipping: go is unavailable");
    false
}

struct GoProject;

impl GoProject {
    const ROUTE: &str = r#"import (
	"io"
	"net/http"
	"strings"
)

// @RequestRoute("/greet/{name}")
// @Public
type GreetController struct{}

// @GetRoute
func (GreetController) Get(
	// @FromPath
	name string,
) map[string]string {
	return map[string]string{"hello": name}
}

// @PostRoute
func (GreetController) Post(request *http.Request) *http.Response {
	body, _ := io.ReadAll(request.Body)
	answer := `{"created":"` + string(body) + `"}`
	header := http.Header{"Content-Type": {"application/json"}}
	return &http.Response{StatusCode: http.StatusCreated, Header: header, Body: io.NopCloser(strings.NewReader(answer))}
}
"#;
    const MIDDLEWARE: &str = r#"import (
	"io"
	"net/http"
	"strings"
)

type Middleware struct{}

func (Middleware) Before(request *http.Request) *http.Response {
	if request.Header.Get("x-deny") != "" {
		header := http.Header{"Content-Type": {"application/json"}}
		body := io.NopCloser(strings.NewReader(`{"error":"denied"}`))
		return &http.Response{StatusCode: http.StatusUnauthorized, Header: header, Body: body}
	}
	return nil
}

func (Middleware) After(request *http.Request) *http.Response {
	header := http.Header{"X-Middleware-After": {request.Method}}
	return &http.Response{StatusCode: http.StatusNoContent, Header: header}
}
"#;

    fn worker(marker: &Path) -> String {
        format!(
            "import \"os\"\n\ntype Handler struct{{}}\n\nfunc (Handler) POST() {{\n\tos.WriteFile(`{}`, []byte(\"go worker\\n\"), 0o600)\n}}\n",
            marker.display()
        )
    }

    fn write(root: &Path, marker: &Path) {
        write(&root.join("client/pages/tac.html"), "<test-view30 />");
        write(
            &root.join("client/components/test/view30/tac.html"),
            "<main aria-label=\"Go\"><h1>Go</h1></main>",
        );
        write(
            &root.join("server/controllers/GreetController.go"),
            Self::ROUTE,
        );
        write(&root.join("middleware.go"), Self::MIDDLEWARE);
        write(&root.join("server/workers/beat.go"), &Self::worker(marker));
        write(
            &root.join(".tachyonrc"),
            r#"{"workers":{"server/workers/beat.go":{"every_seconds":1}}}"#,
        );
    }
}

async fn go_worker_marker(path: &Path) -> String {
    tokio::time::timeout(Duration::from_secs(8), async {
        loop {
            if let Ok(contents) = fs::read_to_string(path)
                && contents.ends_with('\n')
            {
                break contents;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("the Go worker ran on its one-second schedule")
}

#[tokio::test]
async fn a_go_route_middleware_and_worker_run_under_the_server() {
    if !go_usable() {
        return;
    }
    let project = tempfile::tempdir().expect("project");
    let marker_directory = tempfile::tempdir().expect("marker directory");
    let marker = marker_directory.path().join("go-worker");
    GoProject::write(project.path(), &marker);
    let server = DevServer::bind(
        project.path(),
        &DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        },
    )
    .await
    .expect("bind");
    let port = server.address().port();
    let (stop, wait) = tokio::sync::oneshot::channel::<()>();
    let running = tokio::spawn(async move {
        server
            .run_until(async {
                let _stopped = wait.await;
            })
            .await
    });

    let greeted = get(port, "/greet/Ada").await;
    assert!(greeted.starts_with("HTTP/1.1 200"), "{greeted}");
    assert!(greeted.contains(r#"{"hello":"Ada"}"#), "{greeted}");
    assert!(greeted.contains("x-middleware-after: GET"), "{greeted}");

    let options = request(
        port,
        "OPTIONS /greet/Ada HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(options.starts_with("HTTP/1.1 200"), "{options}");
    assert!(
        options.contains("allow: GET, HEAD, OPTIONS, POST"),
        "{options}"
    );
    assert!(options.contains("x-middleware-after: OPTIONS"), "{options}");

    let posted = request(
        port,
        "POST /greet/Ada HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Length: 3\r\nConnection: close\r\n\r\nAda",
    )
    .await;
    assert!(posted.starts_with("HTTP/1.1 201"), "{posted}");
    assert!(posted.contains(r#"{"created":"Ada"}"#), "{posted}");

    let denied = request(
        port,
        "GET /greet/Ada HTTP/1.1\r\nHost: 127.0.0.1\r\nx-deny: yes\r\nConnection: close\r\n\r\n",
    )
    .await;
    assert!(denied.starts_with("HTTP/1.1 401"), "{denied}");
    assert!(!denied.contains("hello"), "{denied}");

    assert_eq!(go_worker_marker(&marker).await, "go worker\n");

    let _stopped = stop.send(());
    let _joined = running.await;
}

#[tokio::test]
async fn a_native_middleware_answer_cannot_set_the_body_length() {
    let project = tempfile::tempdir().expect("project");
    write_middleware_project(
        project.path(),
        (
            "server/controllers/SecretController.js",
            "@RequestRoute('/secret')\n@Public\nexport class SecretController {\n  @GetRoute\n  getSecret() {\n    return { secret: 42 }\n  }\n}\n",
        ),
        (
            "middleware.js",
            "export class Middleware {\n  static after() {\n    return new Response(null, { status: 204, headers: { 'content-length': '0', 'x-after': 'yes' } })\n  }\n}\n",
        ),
    );
    let answer = get_once(project.path(), "/secret").await;
    assert!(answer.starts_with("HTTP/1.1 200"), "{answer}");
    assert!(answer.contains(r#"{"secret":42}"#), "{answer}");
    assert!(answer.contains("x-after: yes"), "{answer}");
}
