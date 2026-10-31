#![allow(clippy::expect_used)]

use super::outgoing_loopback::LoopbackProbe;
use std::sync::atomic::Ordering;
use tachyon_core::ServerKind;

struct RawSocketProbe;

impl RawSocketProbe {
    const REACHED: &str = "reached-directly";
    const PORT: &str = "PRIVATE_PORT";
    const JAVASCRIPT: &str = r"import net from 'node:net'

@RequestRoute('/probe')
@Public
export class ProbeController {
  @GetRoute
  async get() {
    return await new Promise((resolve) => {
      const socket = net.connect(PRIVATE_PORT, '127.0.0.1')
      socket.once('connect', () => { socket.destroy(); resolve('reached-directly') })
      socket.once('error', (error) => resolve(`refused ${error.code}`))
    })
  }
}
";
    const TYPESCRIPT: &str = r"import * as net from 'node:net'

@RequestRoute('/probe')
@Public
class ProbeController {
  @GetRoute
  async get(): Promise<string> {
    return await new Promise<string>((resolve) => {
      const socket = net.connect(PRIVATE_PORT, '127.0.0.1')
      socket.once('connect', () => { socket.destroy(); resolve('reached-directly') })
      socket.once('error', (error: NodeJS.ErrnoException) => resolve(`refused ${error.code}`))
    })
  }
}
";
    const PYTHON: &str = r#"import socket


@request_route("/probe")
@public
class ProbeController:
    @get_route
    def get(self) -> str:
        try:
            socket.create_connection(("127.0.0.1", PRIVATE_PORT), timeout=5).close()
            return "reached-directly"
        except OSError as error:
            return f"refused {error}"
"#;
    const RUBY: &str = r#"require 'socket'

#[RequestRoute("/probe")]
#[Public]
class ProbeController
  # @return [String]
  #[GetRoute]
  def get
    TCPSocket.new('127.0.0.1', PRIVATE_PORT).close
    'reached-directly'
  rescue SystemCallError => error
    "refused #{error.message}"
  end
end
"#;
    const GO: &str = r#"import (
	"net"
)

// @RequestRoute("/probe")
// @Public
type ProbeController struct {
}

// @GetRoute
func (controller *ProbeController) GetProbe() string {
	connection, err := net.Dial("tcp", "127.0.0.1:PRIVATE_PORT")
	if err != nil {
		return "refused " + err.Error()
	}
	connection.Close()
	return "reached-directly"
}
"#;
    const PHP: &str = r#"<?php
#[RequestRoute("/probe")]
#[Public]
class ProbeController {
    #[GetRoute]
    public function get(): string {
        $socket = @stream_socket_client('tcp://127.0.0.1:PRIVATE_PORT', $code, $message, 5);
        if ($socket === false) {
            return "refused $message";
        }
        fclose($socket);
        return 'reached-directly';
    }
}
"#;
    const JAVA: &str = r#"@RequestRoute("/probe")
@Public
class ProbeController {
    @GetRoute
    String get() {
        try (var socket = new java.net.Socket()) {
            socket.connect(new java.net.InetSocketAddress("127.0.0.1", PRIVATE_PORT), 5000);
            return "reached-directly";
        } catch (java.io.IOException error) {
            return "refused " + error;
        }
    }
}
"#;
    const KOTLIN: &str = r#"@RequestRoute("/probe")
@Public
class ProbeController {
    @GetRoute
    fun get(): String {
        return try {
            java.net.Socket().use { it.connect(java.net.InetSocketAddress("127.0.0.1", PRIVATE_PORT), 5000) }
            "reached-directly"
        } catch (error: java.io.IOException) {
            "refused $error"
        }
    }
}
"#;
    const GROOVY: &str = r"@RequestRoute('/probe')
@Public
class ProbeController {
    @GetRoute
    String get() {
        try {
            new Socket().withCloseable { it.connect(new InetSocketAddress('127.0.0.1', PRIVATE_PORT), 5000) }
            return 'reached-directly'
        } catch (IOException error) {
            return 'refused ' + error
        }
    }
}
";
    const CSHARP: &str = r#"[RequestRoute("/probe")]
[Public]
class ProbeController {
    [GetRoute]
    public string Get() {
        try {
            using var client = new System.Net.Sockets.TcpClient();
            client.Connect("127.0.0.1", PRIVATE_PORT);
            return "reached-directly";
        } catch (System.Net.Sockets.SocketException error) {
            return "refused " + error.Message;
        }
    }
}
"#;
    const FSHARP: &str = r#"[<RequestRoute("/probe")>]
[<Public>]
type ProbeController() =
    [<GetRoute>]
    member this.Get() : string =
        try
            use client = new System.Net.Sockets.TcpClient()
            client.Connect("127.0.0.1", PRIVATE_PORT)
            "reached-directly"
        with :? System.Net.Sockets.SocketException as error ->
            "refused " + error.Message
"#;
    const RUST: &str = r#"#[RequestRoute("/probe")]
#[Public]
struct ProbeController {}
impl ProbeController {
    fn new() -> Self { Self {} }

    #[GetRoute]
    fn get(&self) -> String {
        match std::net::TcpStream::connect(("127.0.0.1", PRIVATE_PORT)) {
            Ok(_) => String::from("reached-directly"),
            Err(error) => format!("refused {error}"),
        }
    }
}
"#;
    const DART: &str = r#"import 'dart:io';

@RequestRoute("/probe")
@Public
class ProbeController {
  @GetRoute
  Future<String> get() async {
    try {
      final socket = await Socket.connect('127.0.0.1', PRIVATE_PORT, timeout: const Duration(seconds: 5));
      socket.destroy();
      return 'reached-directly';
    } on SocketException catch (error) {
      return 'refused $error';
    }
  }
}
"#;
    const CPP: &str = r#"#pragma once
#include <cerrno>
#include <cstring>
#include <string>
#include <arpa/inet.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>

class [[RequestRoute("/probe")]] [[Public]] ProbeController {
public:
    [[GetRoute]]
    std::string get() {
        int handle = ::socket(AF_INET, SOCK_STREAM, 0);
        if (handle < 0) {
            return std::string("refused ") + std::strerror(errno);
        }
        sockaddr_in address{};
        address.sin_family = AF_INET;
        address.sin_port = htons(PRIVATE_PORT);
        address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
        int connected = ::connect(handle, reinterpret_cast<sockaddr*>(&address), sizeof(address));
        int failure = errno;
        ::close(handle);
        if (connected != 0) {
            return std::string("refused ") + std::strerror(failure);
        }
        return "reached-directly";
    }
};
"#;
    const LUA: &str = r#"-- @RequestRoute("/probe")
-- @Public
---@class ProbeController
ProbeController = {}

---@return string
-- @GetRoute
function ProbeController:get_probe()
  local pipe = io.popen("curl -s -o /dev/null --noproxy '*' -m 5 http://127.0.0.1:PRIVATE_PORT/ ; echo $?")
  if not pipe then
    return "refused without a process"
  end
  local status = pipe:read("*a")
  pipe:close()
  if status:match("^0") then
    return "reached-directly"
  end
  return "refused " .. status
end
"#;

    const REQUIRED: &str = "YON_EGRESS_REQUIRE_CONFINEMENT";

    fn assert_unconfinable(language: &str, refusal: &str) {
        assert_ne!(
            std::env::consts::OS,
            "macos",
            "{language}: macOS must always confine handlers: {refusal}"
        );
        assert_ne!(
            std::env::var(Self::REQUIRED).as_deref(),
            Ok("1"),
            "{language}: {} is set, but this host cannot confine handlers: {refusal}",
            Self::REQUIRED
        );
        assert!(refusal.contains("TY2113"), "{language}: {refusal}");
        assert!(
            refusal.contains("cannot confine the network access of handler processes: "),
            "{language}: the refusal names no reason: {refusal}"
        );
    }

    async fn check(language: &str, program: &str, file: &str, template: &str) {
        Self::check_server(language, program, (file, template), ServerKind::Development).await;
    }

    async fn check_server(
        language: &str,
        program: &str,
        (file, template): (&str, &str),
        kind: ServerKind,
    ) {
        if !LoopbackProbe::available(program) {
            eprintln!("skipping {language}: {program} is unavailable");
            return;
        }
        let (port, connections) = LoopbackProbe::private_service().await;
        let source = template.replace(Self::PORT, &port.to_string());
        if let Some(refusal) = LoopbackProbe::production_refusal(file, &source).await {
            Self::assert_unconfinable(language, &refusal);
            return;
        }
        let answer =
            LoopbackProbe::answer_with(file, &source, LoopbackProbe::DATA_SOURCE, "unused", kind)
                .await;
        assert!(answer.contains("200 OK"), "{language}: {answer}");
        assert!(answer.contains("refused"), "{language}: {answer}");
        assert!(
            !answer.contains(Self::REACHED),
            "{language} opened a raw socket to an unlisted loopback port: {answer}"
        );
        assert_eq!(
            connections.load(Ordering::SeqCst),
            0,
            "{language}: {answer}"
        );
    }
}

#[tokio::test]
async fn a_javascript_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check(
        "JavaScript",
        "node",
        "ProbeController.js",
        RawSocketProbe::JAVASCRIPT,
    )
    .await;
}

#[tokio::test]
async fn a_production_server_confines_its_handlers() {
    RawSocketProbe::check_server(
        "JavaScript",
        "node",
        ("ProbeController.js", RawSocketProbe::JAVASCRIPT),
        ServerKind::Production,
    )
    .await;
}

#[tokio::test]
async fn a_typescript_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check(
        "TypeScript",
        "node",
        "ProbeController.ts",
        RawSocketProbe::TYPESCRIPT,
    )
    .await;
}

#[tokio::test]
async fn a_python_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check(
        "Python",
        "python3",
        "ProbeController.py",
        RawSocketProbe::PYTHON,
    )
    .await;
}

#[tokio::test]
async fn a_ruby_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("Ruby", "ruby", "ProbeController.rb", RawSocketProbe::RUBY).await;
}

#[tokio::test]
async fn a_go_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("Go", "go", "ProbeController.go", RawSocketProbe::GO).await;
}

#[tokio::test]
async fn a_php_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("PHP", "php-cgi", "ProbeController.php", RawSocketProbe::PHP).await;
}

#[tokio::test]
async fn a_java_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("Java", "java", "ProbeController.java", RawSocketProbe::JAVA).await;
}

#[tokio::test]
async fn a_kotlin_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check(
        "Kotlin",
        "kotlinc",
        "ProbeController.kt",
        RawSocketProbe::KOTLIN,
    )
    .await;
}

#[tokio::test]
async fn a_groovy_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check(
        "Groovy",
        "groovyc",
        "ProbeController.groovy",
        RawSocketProbe::GROOVY,
    )
    .await;
}

#[tokio::test]
async fn a_csharp_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("C#", "dotnet", "ProbeController.cs", RawSocketProbe::CSHARP).await;
}

#[tokio::test]
async fn an_fsharp_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("F#", "dotnet", "ProbeController.fs", RawSocketProbe::FSHARP).await;
}

#[tokio::test]
async fn a_rust_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("Rust", "rustc", "ProbeController.rs", RawSocketProbe::RUST).await;
}

#[tokio::test]
async fn a_dart_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("Dart", "dart", "ProbeController.dart", RawSocketProbe::DART).await;
}

#[tokio::test]
async fn a_cpp_raw_socket_is_refused_by_the_operating_system() {
    RawSocketProbe::check("C++", "clang++", "ProbeController.hpp", RawSocketProbe::CPP).await;
}

#[tokio::test]
async fn a_lua_relay_is_refused_by_the_operating_system() {
    RawSocketProbe::check("Lua", "lua", "ProbeController.lua", RawSocketProbe::LUA).await;
}
