import Foundation
import AppKit
import WebKit

@MainActor final class NativeUiStyleScene {
    private let delegate: AppDelegate
    var view: WKWebView { delegate.fixtureView() }
    var window: NSWindow { delegate.fixtureWindow() }
    var assets: NativeUiStyleAssets { delegate.fixtureAssets() }
    var session: NativeUiStyleSession { delegate.fixtureSession() }

    init(_ installed: NativeUiRuntimeGeneration) { delegate = AppDelegate(installed) }

    func start() async throws {
        NSApplication.shared.delegate = delegate
        delegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))
        try await ready()
        let denied = try await page("const result=JSON.parse(await globalThis.__tachyonNativeHostCall('native.styles.prepare','{}')); return String(result.ok===false)")
        try NativeUiRuntimeFailure.require(denied == "true", "Page obtained native style selection authority")
    }

    func page(_ body: String) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let reply = NativeUiStyleReply(continuation)
            reply.deadline()
            view.callAsyncJavaScript(body, arguments: [:], in: nil, in: .page, completionHandler: { reply.resolve($0) })
        }
    }

    func color() async throws -> String { try await page("return getComputedStyle(document.querySelector('#sample')).color") }
    func stylesheetSnapshot() async throws -> String { try await color() }
    func releaseNativeReply() throws { try NativeUiHostReplyFixture.release() }
    func waitForNativeReply() async throws { try await pending() }

    func routes() async throws {
        let owner = "6F9619FF-8B86-D011-B42D-00C04FC964FD"
        _ = try await invoke(owner, "init")
        let before = try await stopped(owner)
        try await route("status?draft=1#section", "static")
        try await route("topic/telemetry?draft=1", "dynamic")
        try await route("index.html?draft=1", "root")
        _ = try await invoke(owner, "init")
        let after = try await stopped(owner)
        try NativeUiRuntimeFailure.require(after == before + 1, "Generated navigation did not dispose the old native owner exactly once")
        _ = try await invoke(owner, "dispose")
    }

    private func route(_ path: String, _ expected: String) async throws {
        view.load(URLRequest(url: URL(string: "tachyon-app://bundle/" + path)!))
        try await ready(expected)
        let observed = try await page("return document.querySelector('#sample').dataset.route")
        try NativeUiRuntimeFailure.require(observed == expected, "Generated route or query normalization changed")
    }

    private func invoke(_ owner: String, _ operation: String) async throws -> String {
        let payload = "{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"\(owner)\",\"op\":\"\(operation)\"}"
        let request = try JSONSerialization.data(withJSONObject: [payload], options: [])
        let quoted = String(decoding: request, as: UTF8.self).dropFirst().dropLast()
        return try await page("return await globalThis.__tachyonNativeHostCall('companion.invoke',\(quoted))")
    }

    private func stopped(_ owner: String) async throws -> Int {
        let payload = "{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"\(owner)\",\"op\":\"call\",\"name\":\"stopped\",\"args\":[]}"
        let request = try JSONSerialization.data(withJSONObject: [payload], options: [])
        let quoted = String(decoding: request, as: UTF8.self).dropFirst().dropLast()
        let value = try await page("return String(JSON.parse(await globalThis.__tachyonNativeHostCall('companion.invoke',\(quoted))).value)")
        guard let count = Int(value) else { throw NativeUiRuntimeFailure(description: "Native bridge did not expose lifecycle count") }
        return count
    }

    private func ready(_ expected: String = "root") async throws {
        let deadline = Date().addingTimeInterval(10)
        while !(await complete(expected)) {
            try NativeUiRuntimeFailure.require(Date() < deadline, "Generated WKWebView did not become ready")
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    private func complete(_ expected: String) async -> Bool {
        (try? await page("return String(document.readyState==='complete' && document.querySelector('#sample')?.dataset.route==='\(expected)')") ) == "true"
    }

    private func pending() async throws {
        let deadline = Date().addingTimeInterval(10)
        while !NativeUiHostReplyFixture.active() {
            try NativeUiRuntimeFailure.require(Date() < deadline, "Generated native bridge did not receive pending reply")
            try await Task.sleep(nanoseconds: 10_000_000)
        }
    }
}
