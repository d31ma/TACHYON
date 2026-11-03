#!/usr/bin/env node
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

assert.equal(process.platform, 'darwin', 'Apple protocol qualification requires Xcode');
class AppleProtocolTest {
  static repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../..');
  static native = path.join(AppleProtocolTest.repo, 'crates/tachyon-core/src/native');
}

/**
 * @param {string} program
 * @param {string[]} args
 * @param {number} timeout
 */
function run(program, args, timeout) {
  return spawnSync(program, args, {
    encoding: 'utf8', timeout, killSignal: 'SIGKILL', maxBuffer: 1024 * 1024,
  });
}

/** @param {'macos'|'ios'} platform */
function hostBridge(platform) {
  const source = readFileSync(path.join(AppleProtocolTest.native, platform + '.rs'), 'utf8');
  const begin = source.indexOf('    func userContentController(');
  assert.ok(begin >= 0, platform + ' host must retain its guarded bridge entry point');
  const end = source.indexOf('\n    }', begin);
  return `final class ${platform}Bridge {\nvar webView: WKWebView?\n${source.slice(begin, end + 6)}\n` +
    'func handle(_ capability: String, _ payload: String) -> String { tacNativeInvoke(payload) }\n}\n';
}

function companionFixture() {
  return String.raw`
import Foundation
import CoreFoundation

private var count = 1
private var value: Any?
private var calls = 0
func tacServiceKnown(_ service: String) -> Bool { ["probe", "other"].contains(service) }
func tacServiceRequired(_ service: String) -> [String] { [] }
func tacServicePrototype(_ service: String) -> Bool { false }
func tacServiceMembers(_ service: String, _ tacProps: [String: Any]) -> [String: TacMember]? {
    guard tacServiceKnown(service) else { return nil }
    return [
        "count": .field({ service == "probe" ? count : 99 }, { count = $0 as? Int ?? -1 }),
        "value": .field({ value }, { value = $0 }),
        "echo": .method({ arguments in calls += 1; return arguments }),
    ]
}
struct WKUserContentController {}
final class AppleBrowserApis {
    static let shared = AppleBrowserApis()
    func call(_ capability: String, _ request: [String: Any], _ view: WKWebView?, _ reply: @escaping (String) -> Void) {
        reply("{\"value\":\"" + capability + "\"}")
    }
}
final class IosFilePicker {
    static let shared = IosFilePicker()
    func call(_ operation: String, _ request: [String: Any], _ reply: @escaping (String) -> Void) {
        reply("{\"value\":\"" + operation + "\"}")
    }
}
final class WKWebView {}
struct TestRequest { let url: URL? }
struct TestFrame { let isMainFrame: Bool; let request: TestRequest }
struct WKScriptMessage { let webView: WKWebView?; let frameInfo: TestFrame; let body: Any }
func tachyonRoute(_ path: String) -> String? {
    path == "/" || path == "/index.html" ? "/" : path == "/other" ? "/other" : nil
}
func require(_ okay: @autoclosure () -> Bool, _ message: String) {
    if !okay() { FileHandle.standardError.write(Data((message + "\n").utf8)); exit(1) }
}
func response(_ raw: String) -> [String: Any] {
    let answer = tacNativeInvoke(raw)
    guard let parsed = (try? JSONSerialization.jsonObject(with: Data(answer.utf8))) as? [String: Any] else {
        require(false, "Companion produced invalid JSON: " + String(answer.prefix(128))); return [:]
    }
    return parsed
}
func invoke(_ request: [String: Any]) -> [String: Any] {
    let raw = String(data: try! JSONSerialization.data(withJSONObject: request), encoding: .utf8)!
    return response(raw)
}
func bridgeProbe(_ view: WKWebView, _ receive: (WKUserContentController, WKScriptMessage, @escaping (Any?, String?) -> Void) -> Void,
    checksWebView: Bool) {
    func check(_ raw: String, _ url: String = "tachyon-app://bundle/index.html", _ main: Bool = true,
        from sender: WKWebView?? = .none, denied: Bool) {
        var settled = false
        let message = WKScriptMessage(webView: sender ?? view,
            frameInfo: TestFrame(isMainFrame: main, request: TestRequest(url: URL(string: url))),
            body: ["capability": "companion.invoke", "payload": raw])
        receive(WKUserContentController(), message) { answer, error in
            settled = true
            if denied { require(error != nil, "Actual Apple host guard accepted an untrusted or ambiguous request") }
            else {
                require(error == nil, "Actual Apple host guard rejected valid request")
                require((answer as? String) == "{\"value\":1}", "Actual Apple host guard dispatched the wrong route")
            }
        }
        require(settled, "Actual Apple host guard did not settle its reply")
    }
    _ = invoke(["route":"/", "service":"probe", "owner":"00000000-0000-4000-8000-000000000001", "op":"init"])
    let valid = #"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","op":"get","name":"count"}"#
    check(valid, denied: false)
    check(valid, "https://foreign.example/", denied: true)
    check(valid, "tachyon-app://bundle/index.html", false, denied: true)
    check(valid, "tachyon-app://bundle:444/index.html", denied: true)
    check(valid, "tachyon-app://user@bundle/index.html", denied: true)
    if checksWebView {
        check(valid, from: WKWebView(), denied: true)
        check(valid, from: .some(nil), denied: true)
    }
    check(#"{"route":"/other","service":"probe","owner":"00000000-0000-4000-8000-000000000001","op":"get","name":"count"}"#, denied: true)
    check(#"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","route":"/other","op":"get","name":"count"}"#, denied: true)
    check(#"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","\u0072oute":"/other","op":"get","name":"count"}"#, denied: true)
    let browserCalls: [(String, String?)] = [
        ("tachyon-app://bundle/index.html", "{\"value\":\"browserApi.clipboard.readText\"}"), ("https://foreign.example/", nil),
    ]
    for (url, expected) in browserCalls {
        var answered: Any?
        var refused: String?
        let message = WKScriptMessage(webView: view,
            frameInfo: TestFrame(isMainFrame: true, request: TestRequest(url: URL(string: url))),
            body: ["capability": "browserApi.clipboard.readText", "payload": "{}"])
        receive(WKUserContentController(), message) { answer, error in answered = answer; refused = error }
        if let expected { require(answered as? String == expected, "A browser API call did not reach AppleBrowserApis") }
        else { require(refused != nil && answered == nil, "A foreign frame reached AppleBrowserApis") }
    }
}
`;
}

function protocolProbes() {
  return String.raw`
switch CommandLine.arguments.last! {
case "duplicates":
    for raw in [
        #"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","route":"/other","op":"get","name":"count"}"#,
        #"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","\u0072oute":"/other","op":"get","name":"count"}"#,
        #"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","op":"get","op":"call","name":"count"}"#,
    ] { require(response(raw)["error"] != nil, "Ambiguous duplicate protocol keys reached a companion") }
case "unicode":
    _ = invoke(["route":"/", "service":"probe", "owner":"00000000-0000-4000-8000-000000000001", "op":"init"])
    require(response(#"{"́":1,"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","op":"get","name":"count"}"#)["value"] as? Int == 1,
        "Valid combining-character JSON did not complete")
    for text in ["́", "😀", "é", "日本語", "\u{0000}", "\\\"}\"route\":\"/other\""] {
        _ = invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"set", "name":"value", "value":text])
        require(invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"get", "name":"value"])["value"] as? String == text,
            "Unicode or NUL value changed at the Swift boundary")
    }
    _ = response(#"{"route":"/","service":"probe","owner":"00000000-0000-4000-8000-000000000001","op":"set","name":"value","value":"\ud83d\ude00"}"#)
    require(value as? String == "😀", "Escaped surrogate pair was not decoded")
case "values":
    _ = invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"init"])
    require(calls == 0, "Initializing native members called an application method")
    _ = invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"set", "name":"count", "value":7])
    require(count == 7, "Integer field assignment changed its type")
    for number in [0, 1, 7, -2] {
        let result = invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"call", "name":"echo", "args":[number]])["value"] as! [Any]
        require(CFGetTypeID(result[0] as! NSNumber) != CFBooleanGetTypeID(), "Integer became a boolean")
    }
    let result = invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"call", "name":"echo", "args":[true, NSNull(), [NSNull(), 1], ["nil":NSNull()]]])["value"] as! [Any]
    require(result[0] as? Bool == true && result[1] is NSNull, "Boolean/null value changed")
    require((result[2] as? [Any])?.first is NSNull, "Nested array null changed")
    require((result[3] as? [String:Any])?["nil"] is NSNull, "Nested dictionary null disappeared")
    let fractional = invoke(["route":"/", "service":"probe","owner":"00000000-0000-4000-8000-000000000001", "op":"call", "name":"echo", "args":[0.25, -1.5]])["value"] as! [Double]
    require(fractional == [0.25, -1.5], "Fractional values changed")
    let boundary = "{\"route\":\"/\",\"service\":\"probe\",\"owner\":\"00000000-0000-4000-8000-000000000001\",\"op\":\"init\",\"x\":" + String(repeating:"[",count:63) + "0" + String(repeating:"]",count:63) + "}"
    require(response(boundary)["error"] == nil, "Valid depth-64 request was refused")
    let prefix = "{\"route\":\"/\",\"service\":\"probe\",\"owner\":\"00000000-0000-4000-8000-000000000001\",\"op\":\"init\",\"padding\":\""
    let bytesBoundary = prefix + String(repeating:"a",count:65536-prefix.utf8.count-2) + "\"}"
    require(response(bytesBoundary)["error"] == nil, "Valid 64-KiB request was refused")
    for raw in ["{", "[]", "{\"route\":\"/\",}", String(repeating:"x", count:65537),
        "{\"route\":\"/\",\"service\":\"probe\",\"owner\":\"00000000-0000-4000-8000-000000000001\",\"op\":\"init\",\"x\":" + String(repeating:"[",count:65) + "0" + String(repeating:"]",count:65) + "}"] {
        require(response(raw)["error"] != nil, "Malformed/oversized/deep JSON was accepted")
    }
case "bridge":
    let view = WKWebView()
    let macos = macosBridge()
    macos.webView = view
    bridgeProbe(view, macos.userContentController, checksWebView: true)
    let ios = iosBridge()
    ios.webView = view
    bridgeProbe(view, ios.userContentController, checksWebView: false)
default: require(false, "Unknown Apple protocol probe")
}
print("PASS: " + CommandLine.arguments.last!)
`;
}

/** @returns {string} */
function companionPrelude() {
  const host = readFileSync(path.join(AppleProtocolTest.native, 'macos.rs'), 'utf8');
  const begin = host.indexOf('SWIFT_COMPANION_PRELUDE');
  const end = host.indexOf(');', begin);
  assert.ok(begin >= 0 && end > begin, 'the macOS host must keep its Swift companion prelude list');
  const files = [...host.slice(begin, end).matchAll(/include_str!\("([^"]+)"\)/g)].map(match => match[1]);
  assert.ok(files.length > 0, 'the Swift companion prelude list must name its files');
  return files.map(file => readFileSync(path.join(AppleProtocolTest.native, file), 'utf8')).join('\n');
}

/** @param {string} temporary */
function compileProbe(temporary) {
  const helper = path.join(AppleProtocolTest.native, 'apple_json.swift');
  const preludes = readFileSync(helper, 'utf8') + '\n' + companionPrelude();
  const source = path.join(temporary, 'main.swift');
  const binary = path.join(temporary, 'probe');
  writeFileSync(source, preludes + companionFixture() + hostBridge('macos') + hostBridge('ios') + protocolProbes());
  const compiled = run('/usr/bin/xcrun', ['swiftc', source, '-o', binary], 60000);
  assert.equal(compiled.status, 0, String(compiled.error || compiled.stderr));
  return binary;
}

/** @param {string} binary */
function runProbes(binary) {
  let failures = 0;
  for (const probe of ['duplicates', 'unicode', 'values', 'bridge']) {
    const result = run(binary, [probe], 60000);
    if (result.status !== 0) {
      failures++;
      console.error(`FAIL: ${probe}: ${result.error || result.stderr || result.signal}`);
    } else console.log(result.stdout.trim());
  }
  assert.equal(failures, 0, `${failures} Apple protocol security regressions failed`);
}

function main() {
  const temporary = mkdtempSync(path.join(tmpdir(), 'tachyon-apple-protocol-'));
  try {
    const binary = compileProbe(temporary);
    runProbes(binary);
  } finally {
    rmSync(temporary, { recursive: true, force: true });
  }
}

main();
