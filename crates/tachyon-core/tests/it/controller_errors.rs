#![allow(clippy::expect_used)]

use command_group::AsyncCommandGroup as _;
use std::fs;
use std::path::Path;
use std::time::Duration;
use tachyon_core::{DevServer, DevServerOptions, ProjectDiscovery};
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;

#[path = "controller_errors/csharp.rs"]
mod csharp;
#[path = "controller_errors/dart.rs"]
mod dart;
#[path = "controller_errors/groovy.rs"]
mod groovy;
#[path = "controller_errors/java.rs"]
mod java;
#[path = "controller_errors/javascript.rs"]
mod javascript;
#[path = "controller_errors/kotlin.rs"]
mod kotlin;
#[path = "controller_errors/php.rs"]
mod php;
#[path = "controller_errors/python.rs"]
mod python;
#[path = "controller_errors/ruby.rs"]
mod ruby;
#[path = "controller_errors/typescript.rs"]
mod typescript;

struct Commands;

impl Commands {
    async fn success(
        program: &Path,
        arguments: &[&str],
        environment: &[(&str, &str)],
        seconds: u64,
    ) -> bool {
        let mut command = tokio::process::Command::new(program);
        command
            .args(arguments)
            .envs(environment.iter().copied())
            .kill_on_drop(true);
        if seconds < 60 {
            command
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null());
        }
        let Ok(mut child) = command.group_spawn() else {
            return false;
        };
        if let Ok(Ok(status)) =
            tokio::time::timeout(Duration::from_secs(seconds), child.wait()).await
        {
            return status.success();
        }
        let _ = child.kill().await;
        let _ = child.wait().await;
        false
    }

    async fn available(program: &str, extension: &str) -> bool {
        let flag = if program == "kotlinc" {
            "-version"
        } else {
            "--version"
        };
        if Self::success(Path::new(program), &[flag], &[], 15).await {
            return true;
        }
        assert!(std::env::var_os("CI").is_none(), "missing {program} in CI");
        eprintln!("skipping {extension} error handler: missing {program}");
        false
    }
}

struct Files;

impl Files {
    fn write(root: &Path, relative: &str, source: &str) {
        let destination = root.join(relative);
        fs::create_dir_all(destination.parent().expect("parent")).expect("directory");
        fs::write(destination, source).expect("source");
    }
}

struct Http;

impl Http {
    async fn get(port: u16, path: &str) -> String {
        let mut socket = TcpStream::connect(("127.0.0.1", port))
            .await
            .expect("connect");
        let request =
            format!("GET {path} HTTP/1.1\r\nHost: localhost\r\nConnection: close\r\n\r\n");
        socket.write_all(request.as_bytes()).await.expect("request");
        let mut response = Vec::new();
        tokio::time::timeout(Duration::from_secs(30), socket.read_to_end(&mut response))
            .await
            .expect("response deadline")
            .expect("response");
        String::from_utf8(response).expect("UTF-8 response")
    }

    fn json_body(response: &str) -> serde_json::Value {
        let body = response.split_once("\r\n\r\n").expect("body").1;
        serde_json::from_str(body).expect("JSON body")
    }
}

struct Server;

impl Server {
    async fn responses(root: &Path, paths: &[&str]) -> Vec<String> {
        let project = ProjectDiscovery::discover(root).expect("discover");
        let options = DevServerOptions {
            port: 0,
            watch: false,
            ..DevServerOptions::default()
        };
        let server = DevServer::bind_project(&project, &options)
            .await
            .expect("bind");
        let port = server.address().port();
        let (stop, stopped) = tokio::sync::oneshot::channel();
        let running = tokio::spawn(server.run_until(async {
            let _ = stopped.await;
        }));
        let mut responses = Vec::new();
        for path in paths {
            responses.push(Http::get(port, path).await);
        }
        stop.send(()).expect("stop");
        running.await.expect("task").expect("shutdown");
        responses
    }
}

struct Project {
    extension: &'static str,
    program: &'static str,
    files: &'static [(&'static str, &'static str)],
    checks_invalid_model: bool,
}

impl Project {
    const HANDLED: [(&'static str, u16, &'static str); 4] = [
        ("/orders", 404, "missing"),
        ("/orders/gone", 410, "gone"),
        ("/orders/service", 404, "missing"),
        ("/status", 404, "missing"),
    ];
    const SAFE_FAILURES: [&'static str; 2] = ["/orders/unknown", "/orders/broken"];
    const INVALID_MODEL: &'static str = "/orders/invalid";
    const PUBLISHED: [(&'static str, u16); 2] = [("OrderNotFound", 404), ("OrderGone", 410)];

    fn write(&self, root: &Path) {
        for (name, source) in self.files {
            let relative = format!("server/{name}.{}", self.extension);
            Files::write(root, &relative, source);
        }
    }

    fn paths(&self) -> Vec<&'static str> {
        let mut paths = vec!["/api.json"];
        for (path, _, _) in Self::HANDLED {
            paths.push(path);
        }
        paths.extend(Self::SAFE_FAILURES);
        if self.checks_invalid_model {
            paths.push(Self::INVALID_MODEL);
        }
        paths
    }

    async fn verify(&self) {
        if !Commands::available(self.program, self.extension).await {
            return;
        }
        let root = tempfile::tempdir().expect("project");
        self.write(root.path());
        let responses = Server::responses(root.path(), &self.paths()).await;
        self.check_schema(&responses[0]);
        for (index, (path, status, code)) in Self::HANDLED.iter().enumerate() {
            self.check_handled(path, &responses[index + 1], *status, code);
        }
        for (path, response) in self.paths()[Self::HANDLED.len() + 1..]
            .iter()
            .zip(&responses[Self::HANDLED.len() + 1..])
        {
            self.check_safe_failure(path, response);
        }
    }

    fn check_handled(&self, path: &str, response: &str, status: u16, code: &str) {
        let extension = self.extension;
        let status_line = format!("HTTP/1.1 {status}");
        assert!(
            response.starts_with(&status_line),
            "{extension} {path}: {response}"
        );
        assert_eq!(
            Http::json_body(response),
            serde_json::json!({ "code": code }),
            "{extension} {path}"
        );
        assert!(
            !response.contains("PRIVATE_ERROR_SENTINEL"),
            "{extension} {path}"
        );
    }

    fn check_safe_failure(&self, path: &str, response: &str) {
        let extension = self.extension;
        assert!(
            response.starts_with("HTTP/1.1 500"),
            "{extension} {path}: {response}"
        );
        assert!(
            !response.contains("PRIVATE_ERROR_SENTINEL"),
            "{extension} {path}: {response}"
        );
    }

    fn check_schema(&self, response: &str) {
        let schema = Http::json_body(response);
        let mut published = Vec::new();
        for route in schema["routes"].as_array().expect("routes") {
            let Some(errors) = route["methods"]["GET"]["errors"].as_array() else {
                continue;
            };
            for error in errors {
                let name = error["type"].as_str().expect("error type");
                let name = name.trim_start_matches(['&', '*']);
                let status = error["status"].as_u64().expect("error status");
                published.push((name.to_owned(), status));
            }
        }
        for (name, status) in Self::PUBLISHED {
            let found = published.iter().any(|(published_name, published_status)| {
                published_name.ends_with(name) && *published_status == u64::from(status)
            });
            assert!(found, "{}: {name} {status} not in {schema}", self.extension);
        }
    }
}
