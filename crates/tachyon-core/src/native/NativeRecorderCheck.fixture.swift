import AppKit
import WebKit

extension AppDelegate {
    fileprivate var recorderView: WKWebView? { webView }
}

@MainActor
private final class RecorderCheck {
    private let delegate: AppDelegate
    private let fileName = "Tachyon recorder check " + UUID().uuidString

    private let progress = Bundle.main.bundleURL.deletingLastPathComponent().appendingPathComponent("progress.txt")

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    private func note(_ text: String) {
        let line = Data((text + "\n").utf8)
        FileHandle.standardOutput.write(line)
        guard let handle = try? FileHandle(forWritingTo: progress) else { try? line.write(to: progress); return }
        _ = try? handle.seekToEnd()
        try? handle.write(contentsOf: line)
        try? handle.close()
    }

    func run() async throws {
        note("waiting for the page")
        let view = try await ready()
        note("page ready")
        let bridged = try await page(view, "return globalThis.__tachyonBrowserApiBridge.join(',')")
        try require(bridged.hasSuffix(",recorder"), "Page bridge list: " + bridged)
        let unknown = try await call(view, "browserApi.recorder.write", "{\"recording\":\"nobody\",\"position\":0,\"data\":\"AA==\"}")
        try require(unknown.contains("(unknown)"), "A write to no recording was answered: " + unknown)
        let invalid = try await call(view, "browserApi.recorder.open", "{\"recording\":\"bad name\",\"fileName\":\"a\",\"format\":\"mp4\"}")
        try require(invalid.contains("(invalid)"), "A bad recording name was answered: " + invalid)
        note("refusals passed")
        try await cancelled(view)
        try await saved(view)
        note("recorder host check passed")
    }

    private func cancelled(_ view: WKWebView) async throws {
        _ = try await page(view, "window.record('closed'); return 'started'")
        note("asked for the save dialog")
        let dialog = try await panel()
        note("save dialog found: " + String(describing: type(of: dialog)))
        dialog.cancel(nil)
        note("pressed cancel")
        let event = try await event(view, "RecordingFailed")
        try require(event.contains("\"reason\":\"cancelled\""), "A closed save dialog answered: " + event)
        note("closed dialog: cancelled")
    }

    private func saved(_ view: WKWebView) async throws {
        _ = try await page(view, "window.record('meeting', '\(fileName)'); return 'started'")
        let dialog = try await panel()
        let saved = dialog.directoryURL?.appendingPathComponent(fileName + ".mp4")
        note("second save dialog found in " + (dialog.directoryURL?.path ?? "no folder"))
        NSApp.activate(ignoringOtherApps: true)
        let pressed = await Task.detached { RecorderCheck.pressSave(getpid()) }.value
        note("pressed save: " + pressed)
        let started = try await event(view, "RecordingStarted")
        guard let saved else { throw failure("The save dialog has no folder") }
        defer { try? FileManager.default.removeItem(at: saved) }
        try require(started.contains("\"fileName\":\"\(fileName).mp4\""), "Started: " + started)
        try await Task.sleep(nanoseconds: 3_000_000_000)
        _ = try await page(view, "window.stopRecording('meeting'); return 'stopping'")
        let stopped = try await event(view, "RecordingStopped")
        let bytes = try Data(contentsOf: saved)
        try require(stopped.contains("\"bytes\":\(bytes.count)"), "Stopped: " + stopped + " but the file has \(bytes.count) bytes")
        try require(bytes.count > 1000, "The file has only \(bytes.count) bytes")
        try require(bytes.subdata(in: 4..<8) == Data("ftyp".utf8), "The file does not start with an ftyp box")
        note("saved \(bytes.count) bytes of MP4 through the save dialog")
    }

    nonisolated private static func pressSave(_ process: pid_t) -> String {
        guard let button = saveButton(AXUIElementCreateApplication(process), depth: 0) else {
            return "no Save button (Accessibility allowed: \(AXIsProcessTrusted()))"
        }
        return String(AXUIElementPerformAction(button, kAXPressAction as CFString).rawValue)
    }

    nonisolated private static func saveButton(_ element: AXUIElement, depth: Int) -> AXUIElement? {
        guard depth < 16 else { return nil }
        if text(element, kAXRoleAttribute) == kAXButtonRole,
           text(element, kAXIdentifierAttribute) == "OKButton" || text(element, kAXTitleAttribute) == "Save" {
            return element
        }
        var children: CFTypeRef?
        AXUIElementCopyAttributeValue(element, kAXChildrenAttribute as CFString, &children)
        for child in (children as? [AXUIElement]) ?? [] {
            if let found = saveButton(child, depth: depth + 1) { return found }
        }
        return nil
    }

    nonisolated private static func text(_ element: AXUIElement, _ name: String) -> String? {
        var value: CFTypeRef?
        AXUIElementCopyAttributeValue(element, name as CFString, &value)
        return value as? String
    }

    private func panel() async throws -> NSSavePanel {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let sheet = NSApp.windows.compactMap({ $0.attachedSheet as? NSSavePanel }).first { return sheet }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        throw failure("No save dialog opened")
    }

    private func event(_ view: WKWebView, _ name: String) async throws -> String {
        let deadline = Date().addingTimeInterval(15)
        while Date() < deadline {
            let found = try await page(view, "const event = window.events.find((item) => item[0] === '\(name)' || item[0] === 'RecordingFailed'); window.events = window.events.filter((item) => item !== event); return event ? JSON.stringify(event[1]) : ''")
            if !found.isEmpty { return name + " " + found }
            try await Task.sleep(nanoseconds: 50_000_000)
        }
        throw failure("No " + name + " event")
    }

    private func ready() async throws -> WKWebView {
        let deadline = Date().addingTimeInterval(10)
        while Date() < deadline {
            if let view = delegate.recorderView, !view.isLoading, view.title == "Home",
               try await page(view, "return String(typeof window.record)") == "function" { return view }
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        throw failure("Generated host did not load the recorder page")
    }

    private func call(_ view: WKWebView, _ capability: String, _ payload: String) async throws -> String {
        let value = try await view.callAsyncJavaScript(
            "return await globalThis.__tachyonNativeHostCall(capability, payload)",
            arguments: ["capability": capability, "payload": payload],
            contentWorld: .page
        )
        return value as? String ?? String(describing: value)
    }

    private func page(_ view: WKWebView, _ body: String) async throws -> String {
        let value = try await view.callAsyncJavaScript(body, arguments: [:], contentWorld: .page)
        return value as? String ?? String(describing: value)
    }

    private func require(_ valid: Bool, _ message: String) throws {
        if !valid { throw failure(message) }
    }

    private func failure(_ message: String) -> NSError {
        NSError(domain: "NativeRecorderCheck", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
    }
}

@main
private struct RecorderCheckMain {
    static func main() {
        let application = NSApplication.shared
        let delegate = AppDelegate()
        application.delegate = delegate
        application.setActivationPolicy(.regular)
        Task { @MainActor () -> Void in
            do { try await RecorderCheck(delegate).run(); exit(0) }
            catch { FileHandle.standardError.write(Data(("recorder fixture failed: " + error.localizedDescription + "\n").utf8)); exit(1) }
        }
        DispatchQueue.main.asyncAfter(deadline: .now() + 60) { () -> Void in
            FileHandle.standardError.write(Data("recorder fixture deadline\n".utf8)); exit(1)
        }
        withExtendedLifetime(delegate) { (_: AppDelegate) -> Void in application.run() }
    }
}
