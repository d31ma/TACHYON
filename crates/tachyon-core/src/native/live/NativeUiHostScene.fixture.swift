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

    func routes(_ candidate: NativeUiRuntimeGeneration) async throws {
        let owner = "6F9619FF-8B86-D011-B42D-00C04FC964FD"
        _ = try await invoke(owner, "init")
        let before = try await stopped(owner)
        try await route("status?draft=1#section", "static")
        try await route("topic/telemetry?draft=1", "dynamic")
        try await route("index.html?draft=1", "root")
        _ = try await invoke(owner, "init")
        let after = try await stopped(owner)
        try NativeUiRuntimeFailure.require(after == before + 1, "Generated navigation did not dispose the old native owner exactly once")
        try await queried(candidate, owner)
        _ = try await invoke(owner, "dispose")
        try await staleNavigation(candidate)
    }

    private func queried(_ candidate: NativeUiRuntimeGeneration, _ owner: String) async throws {
        let identity = try await value(owner, "identity")
        let stops = try await stopped(owner)
        let windowNumber = window.windowNumber
        let marker = try await page("document.querySelector('#field').value='query-kept'; document.querySelector('#field').focus(); return window.marker")
        let address = view.url!.absoluteString
        let ticket = try await session.prepare(candidate)
        try NativeUiRuntimeFailure.require(ticket.url == address, "Query was dropped from the actual document ticket")
        try await session.commit(ticket)
        try NativeUiRuntimeFailure.require(try await color() == "rgb(0, 0, 255)", "Queried document did not commit captured CSS")
        let state = try await page("return JSON.stringify([window.marker,document.querySelector('#field').value,document.activeElement.id])")
        let expected = try JSONSerialization.data(withJSONObject: [marker, "query-kept", "field"], options: [])
        try NativeUiRuntimeFailure.require(state == String(decoding: expected, as: UTF8.self), "Queried document state/focus changed")
        try NativeUiRuntimeFailure.require(view.url!.absoluteString == address && window.windowNumber == windowNumber && window.contentView === view, "Queried style update replaced the native document/window")
        let finalIdentity = try await value(owner, "identity")
        let finalStops = try await stopped(owner)
        try NativeUiRuntimeFailure.require(finalIdentity == identity && finalStops == stops, "Queried style update changed native identity/lifecycle")
        try await session.rollback(ticket)
        try NativeUiRuntimeFailure.require(try await color() == "rgb(0, 0, 0)", "Queried document rollback changed captured styles")
        try await childFrame(candidate)
        let childStops = try await stopped(owner)
        try NativeUiRuntimeFailure.require(childStops == finalStops, "Refused child frame disposed the native root")
    }

    private func childFrame(_ candidate: NativeUiRuntimeGeneration) async throws {
        _ = try await page("const frame=document.createElement('iframe'); frame.id='native-style-frame'; frame.src='tachyon-app://bundle/index.html?child=1'; document.body.append(frame); return 'created'")
        try await delegate.fixtureFrameControl().check(view, candidate, assets)
        _ = try await page("document.querySelector('#native-style-frame').remove(); return 'removed'")
        try NativeUiRuntimeFailure.require(try await color() == "rgb(0, 0, 0)", "Refused child frame changed active main-frame CSS")
    }

    private func staleNavigation(_ candidate: NativeUiRuntimeGeneration) async throws {
        let ticket = try await session.prepare(candidate)
        try await route("status?draft=2", "static")
        let failure = await staleFailure(ticket)
        try NativeUiRuntimeFailure.require(failure != nil, "Old queried-document ticket committed after navigation")
        try NativeUiRuntimeFailure.require(try await color() == "rgb(0, 0, 0)", "Stale navigation changed new document CSS")
    }

    private func staleFailure(_ ticket: NativeUiStyleTicket) async -> Error? {
        do { try await session.commit(ticket); return nil } catch { return error }
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
        let observed = try await value(owner, "stopped")
        guard let count = Int(observed) else { throw NativeUiRuntimeFailure(description: "Native bridge did not expose lifecycle count") }
        return count
    }

    private func value(_ owner: String, _ method: String) async throws -> String {
        let payload = "{\"route\":\"/\",\"service\":\"liveCounter\",\"owner\":\"\(owner)\",\"op\":\"call\",\"name\":\"\(method)\",\"args\":[]}"
        let request = try JSONSerialization.data(withJSONObject: [payload], options: [])
        let quoted = String(decoding: request, as: UTF8.self).dropFirst().dropLast()
        return try await page("return String(JSON.parse(await globalThis.__tachyonNativeHostCall('companion.invoke',\(quoted))).value)")
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
