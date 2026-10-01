import Foundation
import AppKit
import WebKit

@MainActor final class NativeUiStyleScene {
    let view: WKWebView
    let window: NSWindow
    let assets: NativeUiStyleAssets
    let session: NativeUiStyleSession

    init(_ installed: NativeUiRuntimeGeneration) {
        let assets = NativeUiStyleAssets(installed)
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        configuration.setURLSchemeHandler(NativeUiStyleScheme(assets), forURLScheme: "tachyon-app")
        let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 500, height: 300), configuration: configuration)
        let window = NSWindow(contentRect: view.frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
        self.view = view
        self.window = window
        self.assets = assets
        session = NativeUiStyleSession(view, assets)
        window.contentView = view
        window.title = "Tachyon local stylesheet acceptance"
    }

    func start() async throws {
        window.makeKeyAndOrderFront(nil)
        NSApplication.shared.activate(ignoringOtherApps: true)
        view.load(URLRequest(url: URL(string: "tachyon-app://bundle/index.html")!))
        try await ready()
    }

    func page(_ body: String) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let reply = NativeUiStyleReply(continuation)
            reply.deadline()
            view.callAsyncJavaScript(body, arguments: [:], in: nil, in: .page, completionHandler: { reply.resolve($0) })
        }
    }

    func color() async throws -> String { try await page("return getComputedStyle(document.querySelector('#sample')).color") }

    private func ready() async throws {
        let deadline = Date().addingTimeInterval(10)
        while !(await complete()) {
            try NativeUiRuntimeFailure.require(Date() < deadline, "WKWebView fixture did not become ready")
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    private func complete() async -> Bool {
        (try? await page("return String(document.readyState === 'complete' && Boolean(document.querySelector('#sample')))") ) == "true"
    }
}
