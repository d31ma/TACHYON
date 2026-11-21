import AppKit
import WebKit

extension AppDelegate {
    fileprivate var recorderView: WKWebView? { webView }
}

@MainActor
private final class RecorderCheck {
    private let delegate: AppDelegate
    private let folder = FileManager.default.temporaryDirectory.appendingPathComponent("tachyon-recorder-check-" + UUID().uuidString)

    init(_ delegate: AppDelegate) { self.delegate = delegate }

    func run() async throws {
        let view = try await ready()
        try FileManager.default.createDirectory(at: folder, withIntermediateDirectories: true)
        defer { try? FileManager.default.removeItem(at: folder) }
        let bridged = try await page(view, "return globalThis.__tachyonBrowserApiBridge.join(',')")
        try require(bridged.hasSuffix(",recorder"), "Page bridge list: " + bridged)
        let unknown = try await call(view, "browserApi.recorder.write", "{\"recording\":\"nobody\",\"position\":0,\"data\":\"AA==\"}")
        try require(unknown.contains("(unknown)"), "A write to no recording was answered: " + unknown)
        let invalid = try await call(view, "browserApi.recorder.open", "{\"recording\":\"bad name\",\"fileName\":\"a\",\"format\":\"mp4\"}")
        try require(invalid.contains("(invalid)"), "A bad recording name was answered: " + invalid)
        try await cancelled(view)
        try await saved(view)
        FileHandle.standardOutput.write(Data("recorder host check passed\n".utf8))
    }

    private func cancelled(_ view: WKWebView) async throws {
        _ = try await page(view, "window.record('closed'); return 'started'")
        try await panel().cancel(nil)
        let event = try await event(view, "RecordingFailed")
        try require(event.contains("\"reason\":\"cancelled\""), "A closed save dialog answered: " + event)
        FileHandle.standardOutput.write(Data("closed dialog: cancelled\n".utf8))
    }

    private func saved(_ view: WKWebView) async throws {
        _ = try await page(view, "window.record('meeting'); return 'started'")
        let dialog = try await panel()
        dialog.directoryURL = folder
        dialog.nameFieldStringValue = "Fixture.mp4"
        dialog.ok(nil)
        let started = try await event(view, "RecordingStarted")
        try require(started.contains("\"fileName\":\"Fixture.mp4\""), "Started: " + started)
        try await Task.sleep(nanoseconds: 3_000_000_000)
        _ = try await page(view, "window.stopRecording('meeting'); return 'stopping'")
        let stopped = try await event(view, "RecordingStopped")
        let bytes = try Data(contentsOf: folder.appendingPathComponent("Fixture.mp4"))
        try require(stopped.contains("\"bytes\":\(bytes.count)"), "Stopped: " + stopped + " but the file has \(bytes.count) bytes")
        try require(bytes.count > 1000, "The file has only \(bytes.count) bytes")
        try require(bytes.subdata(in: 4..<8) == Data("ftyp".utf8), "The file does not start with an ftyp box")
        FileHandle.standardOutput.write(Data("saved \(bytes.count) bytes of MP4 through the save dialog\n".utf8))
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
