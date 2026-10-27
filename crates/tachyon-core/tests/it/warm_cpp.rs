#![cfg(unix)]
#![allow(clippy::expect_used)]

use crate::support::program_probe::ProgramProbe;
use crate::yon_cpp::CppProject;
use std::time::Duration;
use tachyon_contracts::{
    HandlerBody, HandlerBodyEncoding, HandlerRequest, HandlerResponse, HttpMethod,
};
use tachyon_core::{
    HandlerCancellation, HandlerSource, HandlerSupervisor, HandlerSupervisorOptions, WarmSettings,
    YonIsolationPolicy,
};

struct WarmCpp;

impl WarmCpp {
    const CONTROLLER_PATH: &str = "server/controllers/WarmController.hpp";
    const ECHO_PATH: &str = "server/controllers/EchoController.hpp";
    const MODEL: (&str, &str) = (
        "server/models/WarmCount.hpp",
        "#pragma once\n\nstruct WarmCount {\n    int pid = 0;\n    int count = 0;\n};\n",
    );
    const CONTROLLER: &str = r#"#pragma once
#include <cstdlib>
#include <string>
#include <unistd.h>
#include "../models/WarmCount.hpp"

class [[RequestRoute("/warm")]] [[Public]] WarmController {
public:
    [[PostRoute]]
    WarmCount postWarm(const tachyon::http::HttpRequestMessage& request) {
        std::string mode = request.content.has_value() ? request.content->readAsString() : std::string();
        if (mode == "crash") {
            std::_Exit(23);
        }
        static int served = 0;
        served += 1;
        return WarmCount{static_cast<int>(getpid()), served};
    }

    [[StreamRoute("/ticks")]]
    tachyon::Generator<WarmCount> streamTicks() {
        for (int index = 0; index < 3; index++) {
            co_yield WarmCount{static_cast<int>(getpid()), index};
        }
    }
};
"#;
    const ECHO_FILES: [(&str, &str); 3] = [
        (
            "server/services/EchoService.py",
            "@service\nclass EchoService:\n    def echo(self, value):\n        return {\"value\": value}\n",
        ),
        (
            "server/remotes/EchoClient.hpp",
            "#pragma once\n\nclass [[HTTPClient]] EchoClient {\npublic:\n    [[Gateway({\"class\":\"server/services/EchoService.py\"})]]\n    tachyon::json::JsonNode echo(tachyon::json::JsonNode value) const;\n};\n",
        ),
        (
            "server/controllers/EchoController.hpp",
            r#"#pragma once
#include <unistd.h>

class [[RequestRoute("/echo")]] [[Public]] EchoController {
    [[Needs]] EchoClient& echoClient;

public:
    [[PostRoute]]
    tachyon::json::JsonNode postEcho([[FromBody]] tachyon::json::JsonNode input) {
        static int served = 0;
        served += 1;
        tachyon::json::JsonObject answer;
        answer["echo"] = echoClient.echo(input);
        answer["pid"] = static_cast<int>(getpid());
        answer["count"] = served;
        return answer;
    }
};
"#,
        ),
    ];

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let root = tempfile::tempdir().expect("project");
        for (relative, contents) in files {
            CppProject::write(root.path(), relative, contents);
        }
        root
    }

    fn counter() -> (tempfile::TempDir, HandlerSource) {
        let root = Self::project(&[Self::MODEL, (Self::CONTROLLER_PATH, Self::CONTROLLER)]);
        let source =
            HandlerSource::discover(root.path(), Self::CONTROLLER_PATH).expect("warm controller");
        (root, source)
    }

    fn supervisor() -> HandlerSupervisor {
        let settings = WarmSettings::new(1_000, Duration::from_mins(1));
        HandlerSupervisor::new(HandlerSupervisorOptions {
            isolation: YonIsolationPolicy::Warm(settings),
            default_timeout: Duration::from_secs(10),
            ..HandlerSupervisorOptions::default()
        })
        .expect("warm supervisor")
    }

    fn request(id: &str, route: &str, method: HttpMethod, data: &str) -> HandlerRequest {
        let mut request = HandlerRequest::route(id, route, method);
        request.body = Some(HandlerBody {
            encoding: HandlerBodyEncoding::Utf8,
            data: String::from(data),
        });
        request
    }

    fn body_json(response: &HandlerResponse) -> serde_json::Value {
        let body = response.body.as_ref().expect("successful body");
        serde_json::from_str(&body.data).expect("JSON handler result")
    }

    async fn counted(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        id: &str,
    ) -> (u64, u64) {
        let request = Self::request(id, "/warm", HttpMethod::Post, "");
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("warm response");
        assert!(response.error.is_none(), "{:?}", response.error);
        let value = Self::body_json(&response);
        let pid = value["pid"].as_u64().expect("pid");
        let count = value["count"].as_u64().expect("count");
        (pid, count)
    }

    async fn echoed(
        supervisor: &HandlerSupervisor,
        source: &HandlerSource,
        word: &str,
    ) -> serde_json::Value {
        let input = serde_json::json!(word);
        let mut request = Self::request(word, "/echo", HttpMethod::Post, &input.to_string());
        request.arguments = Some(vec![input]);
        let response = supervisor
            .invoke(source, &request, &HandlerCancellation::default())
            .await
            .expect("echo response");
        assert!(response.error.is_none(), "{:?}", response.error);
        Self::body_json(&response)
    }

    fn compiler_ready() -> bool {
        let ready = CppProject::compiler_available();
        if !ready {
            eprintln!("skipping: clang++ is not installed");
        }
        ready
    }
}

#[tokio::test]
async fn a_warm_cpp_handler_serves_later_requests_from_the_same_process() {
    if !WarmCpp::compiler_ready() {
        return;
    }
    let (_root, source) = WarmCpp::counter();
    let supervisor = WarmCpp::supervisor();
    let (first_pid, first_count) = WarmCpp::counted(&supervisor, &source, "warm_first").await;
    let (second_pid, second_count) = WarmCpp::counted(&supervisor, &source, "warm_second").await;
    assert_eq!(first_pid, second_pid, "the C++ handler reused its process");
    assert_eq!(first_count, 1);
    assert_eq!(second_count, 2, "the C++ handler kept its static state");
}

#[tokio::test]
async fn a_cpp_crash_replaces_the_warm_process() {
    if !WarmCpp::compiler_ready() {
        return;
    }
    let (_root, source) = WarmCpp::counter();
    let supervisor = WarmCpp::supervisor();
    let (before, _) = WarmCpp::counted(&supervisor, &source, "before_crash").await;
    let crashed = supervisor
        .invoke(
            &source,
            &WarmCpp::request("crash", "/warm", HttpMethod::Post, "crash"),
            &HandlerCancellation::default(),
        )
        .await
        .expect_err("crash");
    assert!(crashed.to_string().contains("TY2104"), "{crashed}");
    let (after, count) = WarmCpp::counted(&supervisor, &source, "after_crash").await;
    assert_ne!(before, after, "a new C++ process started");
    assert_eq!(count, 1, "the new C++ process started with fresh state");
}

#[tokio::test]
async fn a_warm_cpp_stream_ends_with_its_end_frame_and_stays_warm() {
    if !WarmCpp::compiler_ready() {
        return;
    }
    let (_root, source) = WarmCpp::counter();
    let supervisor = WarmCpp::supervisor();
    let mut pids = Vec::new();
    for id in ["stream_first", "stream_second"] {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(8);
        let request = WarmCpp::request(id, "/warm/ticks", HttpMethod::Get, "");
        let invocation = supervisor.invoke_streaming(&source, &request, sender);
        let collect = async {
            let mut events = Vec::new();
            while let Some(event) = receiver.recv().await {
                events.push(event);
            }
            events
        };
        let (result, events) = tokio::join!(invocation, collect);
        result.expect("stream finished");
        assert_eq!(events.len(), 3, "{id}");
        let first: serde_json::Value = serde_json::from_str(&events[0].data).expect("event");
        pids.push(first["pid"].as_u64().expect("pid"));
    }
    assert_eq!(pids[0], pids[1]);
    let (after, count) = WarmCpp::counted(&supervisor, &source, "after_streams").await;
    assert_eq!(
        after, pids[0],
        "the C++ process stayed warm after its streams"
    );
    assert_eq!(count, 1);
}

#[tokio::test]
async fn a_warm_cpp_gateway_exchanges_resource_frames_on_every_request() {
    if !WarmCpp::compiler_ready() {
        return;
    }
    if !ProgramProbe::answers("python3", "--version") {
        eprintln!("skipping: python3 is not installed");
        return;
    }
    let root = WarmCpp::project(&WarmCpp::ECHO_FILES);
    let source = HandlerSource::discover(root.path(), WarmCpp::ECHO_PATH).expect("echo caller");
    let supervisor = WarmCpp::supervisor();
    let first = WarmCpp::echoed(&supervisor, &source, "first").await;
    let second = WarmCpp::echoed(&supervisor, &source, "second").await;
    assert_eq!(first["echo"]["value"], "first", "{first}");
    assert_eq!(second["echo"]["value"], "second", "{second}");
    assert_eq!(
        first["pid"], second["pid"],
        "the C++ caller reused its process"
    );
    assert_eq!(first["count"], 1, "{first}");
    assert_eq!(second["count"], 2, "{second}");
}
