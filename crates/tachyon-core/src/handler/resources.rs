mod access;
mod piped_resources;
mod resource_request;

pub(super) use piped_resources::PipedResources;

use super::gateway::GatewayCalls;
use super::process::HandlerCancellation;
use super::topic_bus::TopicOutbox;
use crate::Failure;
use crate::failure::diagnostic;
use crate::storage::ManagedStorage;
use access::Access;
use std::ffi::{OsStr, OsString};
use std::fmt::Write as _;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio::task::JoinHandle;
use tokio::time::Instant;

pub(super) struct ResourceAccess {
    endpoint: String,
    token: String,
    topics: Option<Arc<TopicOutbox>>,
    gateways: Option<Arc<GatewayCalls>>,
    stop: HandlerCancellation,
    task: JoinHandle<()>,
}

impl ResourceAccess {
    const MAX_BYTES: usize = 1024 * 1024;
    const MAX_HEAD_BYTES: usize = 8192;
    const MAX_CONNECTIONS: usize = 4;
    const LOOPBACK_HOST: &str = "127.0.0.1";
    const PROXY_BYPASS_NAMES: [&str; 2] = ["NO_PROXY", "no_proxy"];

    #[cfg(feature = "fuzzing")]
    pub(super) fn fuzz_headers(bytes: &[u8]) {
        let fits = bytes.len() <= Self::MAX_HEAD_BYTES;
        let _result = fits.then(|| resource_request::ResourceRequest::headers(bytes));
    }

    pub(super) async fn bind(
        storage: Option<Arc<ManagedStorage>>,
        gateways: Option<GatewayCalls>,
        topics: Option<TopicOutbox>,
        stop: HandlerCancellation,
        deadline: Instant,
    ) -> Result<Self, Failure> {
        let listener = TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_| Self::unavailable())?;
        let address = listener.local_addr().map_err(|_| Self::unavailable())?;
        let token = Self::random_token()?;
        let topics = topics.map(Arc::new);
        let gateways = gateways.map(Arc::new);
        let access = Access {
            storage,
            gateways: gateways.clone(),
            topics: topics.clone(),
            token: token.clone(),
            host: address.to_string(),
            deadline,
            stop: stop.clone(),
        };
        let task = access.spawn(listener);
        Ok(Self {
            endpoint: format!("http://{address}/"),
            token,
            topics,
            gateways,
            stop,
            task,
        })
    }

    pub(super) fn topics(&self) -> Option<Arc<TopicOutbox>> {
        self.topics.clone()
    }

    pub(super) fn piped(&self) -> PipedResources {
        PipedResources {
            gateways: self.gateways.clone(),
        }
    }

    pub(super) async fn close(mut self) {
        self.stop.cancel();
        let _finished = (&mut self.task).await;
    }

    pub(super) fn apply(&self, command: &mut Command) {
        command.env("YON_RESOURCE_ENDPOINT", &self.endpoint);
        command.env("YON_RESOURCE_TOKEN", &self.token);
        Self::add_loopback_bypasses(command);
    }

    fn add_loopback_bypasses(command: &mut Command) {
        for name in Self::PROXY_BYPASS_NAMES {
            Self::add_loopback_bypass(command, name);
        }
    }

    fn add_loopback_bypass(command: &mut Command, name: &str) {
        let bypass = Self::loopback_proxy_bypass(command.as_std(), name);
        command.env(name, bypass);
    }

    fn loopback_proxy_bypass(command: &std::process::Command, name: &str) -> OsString {
        let mut bypass = OsString::new();
        let values = command
            .get_envs()
            .filter_map(|(key, value)| Self::named_value(key, value, name));
        for value in values {
            bypass.push(value);
            bypass.push(",");
        }
        bypass.push(Self::LOOPBACK_HOST);
        bypass
    }

    fn named_value<'a>(key: &OsStr, value: Option<&'a OsStr>, name: &str) -> Option<&'a OsStr> {
        if key != name {
            return None;
        }
        value.filter(|value| !value.is_empty())
    }

    fn random_token() -> Result<String, Failure> {
        let mut entropy = [0_u8; 32];
        getrandom::fill(&mut entropy).map_err(|_| Self::unavailable())?;
        let mut token = String::with_capacity(64);
        for byte in entropy {
            let _result = write!(token, "{byte:02x}");
        }
        Ok(token)
    }

    fn unavailable() -> Failure {
        Failure::one(diagnostic(
            2010,
            "Cannot initialize Yon managed resource access.",
            None,
            None,
        ))
    }
}

impl Drop for ResourceAccess {
    fn drop(&mut self) {
        self.stop.cancel();
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use super::resource_request::ResourceRequest;
    use super::*;
    use serde_json::json;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    #[test]
    fn framing_rejects_ambiguous_lengths_and_request_methods() {
        for request in [
            "GET / HTTP/1.1\r\nContent-Length: 0\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: 0\r\ncontent-length: 0\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: 0\r\nTransfer-Encoding: chunked\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: 1048577\r\n\r\n",
            "POST / HTTP/1.1\r\nContent-Length: +2\r\n\r\n",
            "POST / HTTP/1.1\r\n\r\n",
        ] {
            assert!(ResourceRequest::headers(request.as_bytes()).is_err());
        }
    }

    fn listener_host(access: &ResourceAccess) -> String {
        access
            .endpoint
            .trim_start_matches("http://")
            .trim_end_matches('/')
            .to_owned()
    }

    async fn read_response_head(stream: &mut TcpStream) -> Vec<u8> {
        let mut response = Vec::new();
        while !response.ends_with(b"\r\n\r\n") {
            assert!(response.len() < 8192);
            response.push(stream.read_u8().await.expect("response header"));
        }
        response
    }

    fn response_content_length(response: &[u8]) -> usize {
        let headers = std::str::from_utf8(response).expect("HTTP headers");
        headers
            .split("\r\n")
            .find_map(|line| line.strip_prefix("Content-Length: "))
            .expect("response length")
            .parse()
            .expect("decimal length")
    }

    struct RequestHead<'a> {
        host: &'a str,
        token: &'a str,
        content_type: &'a str,
        extra: &'a str,
    }

    fn sql_request(head: &RequestHead<'_>) -> String {
        let body =
            r#"{"operation":"sql","mode":"rows","sql":"SELECT 42 AS answer","parameters":[]}"#;
        format!(
            "POST / HTTP/1.1\r\nHost: {}\r\nAuthorization: Bearer {}\r\nContent-Type: {}\r\nContent-Length: {}\r\n{}\r\n{body}",
            head.host,
            head.token,
            head.content_type,
            body.len(),
            head.extra
        )
    }

    async fn call(access: &ResourceAccess, token: &str, origin: &str) -> String {
        let host = listener_host(access);
        let head = RequestHead {
            host: &host,
            token,
            content_type: "application/json",
            extra: origin,
        };
        send(access, &sql_request(&head)).await
    }

    async fn send(access: &ResourceAccess, request: &str) -> String {
        let host = listener_host(access);
        let mut stream = TcpStream::connect(&host).await.expect("listener");
        stream.write_all(request.as_bytes()).await.expect("request");
        let mut response = read_response_head(&mut stream).await;
        let length = response_content_length(&response);
        assert!(length < 8192);
        let mut body = vec![0; length];
        stream.read_exact(&mut body).await.expect("response body");
        response.extend(body);
        String::from_utf8(response).expect("JSON response")
    }

    async fn sqlite_storage() -> Arc<ManagedStorage> {
        let storage = ManagedStorage::connect(
            &json!({"yon":{"datasource":{"driver":"sqlite","url":"sqlite::memory:"}}}),
        )
        .await
        .expect("storage");
        Arc::new(storage)
    }

    async fn bind_storage(storage: &Arc<ManagedStorage>, deadline: Instant) -> ResourceAccess {
        let stop = HandlerCancellation::default();
        ResourceAccess::bind(Some(Arc::clone(storage)), None, None, stop, deadline)
            .await
            .expect("access")
    }

    async fn is_forbidden(access: &ResourceAccess, token: &str, origin: &str) -> bool {
        call(access, token, origin)
            .await
            .starts_with("HTTP/1.1 403 ")
    }

    #[tokio::test]
    async fn requests_with_another_host_or_content_type_are_forbidden() {
        let storage = sqlite_storage().await;
        let access = bind_storage(&storage, Instant::now() + Duration::from_secs(5)).await;
        let host = listener_host(&access);
        let wrong_host = RequestHead {
            host: "localhost",
            token: &access.token,
            content_type: "application/json",
            extra: "",
        };
        let wrong_type = RequestHead {
            host: &host,
            token: &access.token,
            content_type: "text/plain",
            extra: "",
        };
        for head in [wrong_host, wrong_type] {
            let response = send(&access, &sql_request(&head)).await;
            assert!(response.starts_with("HTTP/1.1 403 "), "{response}");
        }
        drop(access);
        storage.close().await;
    }

    #[tokio::test]
    async fn a_request_body_over_one_mebibyte_is_refused_before_reading_it() {
        let storage = sqlite_storage().await;
        let access = bind_storage(&storage, Instant::now() + Duration::from_secs(5)).await;
        let host = listener_host(&access);
        let request = format!(
            "POST / HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            access.token,
            ResourceAccess::MAX_BYTES + 1
        );
        let response = send(&access, &request).await;
        assert!(response.starts_with("HTTP/1.1 413 "), "{response}");
        drop(access);
        storage.close().await;
    }

    #[tokio::test]
    async fn capabilities_are_private_distinct_and_revoked_with_the_invocation() {
        let storage = sqlite_storage().await;
        let deadline = Instant::now() + Duration::from_secs(5);
        let access = bind_storage(&storage, deadline).await;
        let other = bind_storage(&storage, deadline).await;
        assert_ne!(access.token, other.token);
        let response = call(&access, &access.token, "").await;
        assert!(response.starts_with("HTTP/1.1 200 "));
        assert!(response.ends_with(r#"[{"answer":42}]"#));
        assert!(is_forbidden(&access, &other.token, "").await);
        let origin = "Origin: http://example.invalid\r\n";
        assert!(is_forbidden(&access, &access.token, origin).await);
        let host = listener_host(&access);
        let mut stalled = TcpStream::connect(&host).await.expect("stalled connection");
        stalled.write_all(b"POST /").await.expect("partial header");
        tokio::task::yield_now().await;
        drop(access);
        let mut bytes = [0; 1];
        let ended = tokio::time::timeout(Duration::from_secs(1), stalled.read(&mut bytes))
            .await
            .expect("revocation is bounded");
        assert!(!matches!(ended, Ok(1..)));
        assert!(TcpStream::connect(&host).await.is_err());
        drop(other);
        storage.close().await;
    }

    #[cfg(unix)]
    mod gateway_lifecycle {
        use super::*;
        use crate::behavior::GatewayLimits;
        use crate::handler::gateway::GatewayCallSetup;
        use crate::handler::{HandlerSource, HandlerSupervisor, HandlerSupervisorOptions};
        use std::path::Path;
        use std::process::{Command as ProcessCommand, Stdio};

        struct GatewayFixture {
            root: tempfile::TempDir,
            source: HandlerSource,
        }

        impl GatewayFixture {
            fn new() -> Self {
                let root = tempfile::tempdir().expect("gateway project");
                let pid_file = root.path().join("processes.json");
                let path = serde_json::to_string(&pid_file).expect("path JSON");
                let program = format!(
                    "import json, os, subprocess, sys, time\nchild = subprocess.Popen([sys.executable, '-c', 'import time; time.sleep(30)'])\nwith open({path}, 'w') as output:\n    json.dump([os.getpid(), child.pid], output)\ntime.sleep(30)\n"
                );
                write(root.path(), "server/helpers/wait.py", &program);
                Self::with_client(
                    root,
                    "{\"command\":[\"python3\",\"server/helpers/wait.py\"]}",
                )
            }

            fn unit() -> Self {
                let root = tempfile::tempdir().expect("gateway project");
                let marker = root.path().join("processes.json");
                let path = serde_json::to_string(&marker).expect("path JSON");
                let service = format!(
                    "@service\nclass WaitService:\n    def wait(self, value):\n        with open({path}, 'w') as output:\n            output.write('started')\n        return value\n"
                );
                write(root.path(), "server/services/WaitService.py", &service);
                Self::with_client(root, "{\"class\":\"server/services/WaitService.py\"}")
            }

            fn with_client(root: tempfile::TempDir, target: &str) -> Self {
                write(
                    root.path(),
                    "server/remotes/WaitClient.js",
                    &format!(
                        "@HTTPClient\nclass WaitClient {{\n@Gateway({target})\nasync wait(value) {{}}\n}}\n"
                    ),
                );
                write(
                    root.path(),
                    "server/controllers/WaitController.js",
                    "@RequestRoute(\"/wait\")\n@Public\nclass WaitController {\n@GetRoute\ngetWait() { return { ok: true }; }\n}\n",
                );
                let source =
                    HandlerSource::discover(root.path(), "server/controllers/WaitController.js")
                        .expect("gateway source");
                Self { root, source }
            }

            fn calls(&self, deadline: Instant, stop: &HandlerCancellation) -> GatewayCalls {
                GatewayCalls::new(GatewayCallSetup {
                    supervisor: HandlerSupervisor::new(HandlerSupervisorOptions::default())
                        .expect("supervisor"),
                    table: Arc::clone(self.source.gateways().expect("gateway table")),
                    request_id: String::from("gateway_lifecycle"),
                    working_directory: self.source.execution_working_directory().to_path_buf(),
                    deadline,
                    cancellation: stop.child(),
                })
            }

            async fn bind(&self, deadline: Instant) -> ResourceAccess {
                let stop = HandlerCancellation::default();
                let gateways = self.calls(deadline, &stop);
                ResourceAccess::bind(None, Some(gateways), None, stop, deadline)
                    .await
                    .expect("gateway endpoint")
            }

            fn started(&self) -> bool {
                self.root.path().join("processes.json").exists()
            }

            async fn processes(&self) -> TestProcesses {
                let path = self.root.path().join("processes.json");
                for _attempt in 0..200 {
                    if let Ok(bytes) = std::fs::read(&path)
                        && let Ok(ids) = serde_json::from_slice::<[u32; 2]>(&bytes)
                    {
                        return TestProcesses { ids, gone: false };
                    }
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
                panic!("gateway command did not start");
            }
        }

        struct TestProcesses {
            ids: [u32; 2],
            gone: bool,
        }

        impl TestProcesses {
            async fn assert_gone(mut self) {
                for _attempt in 0..100 {
                    if self.ids.iter().all(|pid| !signal("-0", *pid)) {
                        self.gone = true;
                        return;
                    }
                    tokio::time::sleep(Duration::from_millis(20)).await;
                }
                panic!("gateway processes survived cleanup: {:?}", self.ids);
            }
        }

        impl Drop for TestProcesses {
            fn drop(&mut self) {
                if self.gone {
                    return;
                }
                for pid in self.ids {
                    signal("-KILL", pid);
                }
            }
        }

        fn signal(kind: &str, pid: u32) -> bool {
            ProcessCommand::new("kill")
                .args([kind, &pid.to_string()])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success())
        }

        fn python_available() -> bool {
            let available = ProcessCommand::new("python3")
                .arg("--version")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
            let required = std::env::var_os("CI").is_some();
            assert!(
                available || !required,
                "python3 is required for gateway tests in CI"
            );
            available
        }

        fn write(root: &Path, relative: &str, text: &str) {
            let path = root.join(relative);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("directory");
            std::fs::write(path, text).expect("fixture");
        }

        fn wait_call(argument: &serde_json::Value) -> serde_json::Value {
            json!({"operation":"gateway","key":"WaitClient.wait","arguments":[argument]})
        }

        async fn request(access: &ResourceAccess, large: bool) -> TcpStream {
            let argument = if large {
                "x".repeat(900_000)
            } else {
                String::new()
            };
            request_with(access, &wait_call(&json!(argument))).await
        }

        async fn request_with(access: &ResourceAccess, call: &serde_json::Value) -> TcpStream {
            let host = listener_host(access);
            let mut stream = TcpStream::connect(&host).await.expect("gateway connection");
            let body = call.to_string();
            let head = format!(
                "POST / HTTP/1.1\r\nHost: {host}\r\nAuthorization: Bearer {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
                access.token,
                body.len()
            );
            stream.write_all(head.as_bytes()).await.expect("header");
            stream.write_all(body.as_bytes()).await.expect("body");
            stream
        }

        async fn answer_body(mut stream: TcpStream) -> serde_json::Value {
            let head = read_response_head(&mut stream).await;
            assert!(head.starts_with(b"HTTP/1.1 200 "), "{head:?}");
            let mut body = vec![0; response_content_length(&head)];
            stream.read_exact(&mut body).await.expect("answer body");
            serde_json::from_slice(&body).expect("answer JSON")
        }

        fn nested(depth: usize) -> serde_json::Value {
            let mut value = json!(0);
            for _level in 0..depth {
                value = json!([value]);
            }
            value
        }

        #[tokio::test]
        async fn the_endpoint_refuses_arguments_beyond_the_depth_and_node_limits() {
            if !python_available() {
                return;
            }
            let fixture = GatewayFixture::new();
            let access = fixture.bind(Instant::now() + Duration::from_secs(10)).await;
            let too_deep = nested(GatewayLimits::MAX_DEPTH);
            let too_many = serde_json::Value::Array(vec![json!(0); GatewayLimits::MAX_NODES]);
            for argument in [too_deep, too_many] {
                let stream = request_with(&access, &wait_call(&argument)).await;
                assert_eq!(answer_body(stream).await, json!({"error": "invalid"}));
            }
            assert!(!fixture.started());
            access.close().await;
        }

        #[tokio::test]
        async fn no_time_left_refuses_commands_and_units_without_starting_them() {
            if !python_available() {
                return;
            }
            for fixture in [GatewayFixture::new(), GatewayFixture::unit()] {
                let deadline = Instant::now();
                tokio::time::sleep(Duration::from_millis(5)).await;
                let calls = fixture.calls(deadline, &HandlerCancellation::default());
                let answer = calls.answer(&wait_call(&json!("late"))).await;
                let answer: serde_json::Value = serde_json::from_slice(&answer).expect("JSON");
                assert_eq!(answer, json!({"error": "timeout"}));
                assert!(!fixture.started());
            }
        }

        #[tokio::test]
        async fn dropping_access_reaps_a_gateway_process_group() {
            if !python_available() {
                return;
            }
            let fixture = GatewayFixture::new();
            let access = fixture.bind(Instant::now() + Duration::from_secs(10)).await;
            let _stream = request(&access, false).await;
            let processes = fixture.processes().await;
            drop(access);
            processes.assert_gone().await;
        }

        #[tokio::test]
        async fn simultaneous_gateway_and_endpoint_deadlines_reap_processes() {
            if !python_available() {
                return;
            }
            let fixture = GatewayFixture::new();
            let access = fixture.bind(Instant::now() + Duration::from_secs(2)).await;
            let _stream = request(&access, false).await;
            let processes = fixture.processes().await;
            tokio::time::sleep_until(Instant::now() + Duration::from_secs(2)).await;
            processes.assert_gone().await;
            access.close().await;
        }

        #[tokio::test]
        async fn caller_cancellation_reaps_a_gateway_process_group() {
            if !python_available() {
                return;
            }
            let fixture = GatewayFixture::new();
            let access = fixture.bind(Instant::now() + Duration::from_secs(10)).await;
            let _stream = request(&access, false).await;
            let processes = fixture.processes().await;
            access.stop.cancel();
            processes.assert_gone().await;
            access.close().await;
        }

        #[tokio::test]
        async fn close_joins_gateway_cleanup_with_blocked_stdin() {
            if !python_available() {
                return;
            }
            let fixture = GatewayFixture::new();
            let access = fixture.bind(Instant::now() + Duration::from_secs(10)).await;
            let _stream = request(&access, true).await;
            let processes = fixture.processes().await;
            tokio::time::timeout(Duration::from_secs(3), access.close())
                .await
                .expect("bounded cleanup");
            processes.assert_gone().await;
        }
    }
}
