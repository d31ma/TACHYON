import Foundation
import AppKit
import WebKit

@MainActor final class NativeUiStyleScene {
    let view: WKWebView
    let window: NSWindow
    let assets: NativeUiStyleAssets
    let session: NativeUiStyleSession
    let recorder: NativeUiStyleRecorder

    init(_ installed: NativeUiRuntimeGeneration) {
        let assets = NativeUiStyleAssets(installed)
        let configuration = WKWebViewConfiguration()
        configuration.websiteDataStore = .nonPersistent()
        let recorder = NativeUiStyleRecorder(assets)
        configuration.setURLSchemeHandler(recorder, forURLScheme: "tachyon-app")
        let view = WKWebView(frame: NSRect(x: 0, y: 0, width: 500, height: 300), configuration: configuration)
        let window = NSWindow(contentRect: view.frame, styleMask: [.titled, .closable], backing: .buffered, defer: false)
        self.view = view
        self.window = window
        self.assets = assets
        self.recorder = recorder
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

    func requested(_ path: String) async throws {
        let deadline = Date().addingTimeInterval(10)
        while !recorder.requests.contains(path) {
            try NativeUiRuntimeFailure.require(Date() < deadline, "WebKit did not request \(path); requests=\(recorder.requests)")
            try await Task.sleep(nanoseconds: 50_000_000)
        }
    }

    func color() async throws -> String { try await page("return getComputedStyle(document.querySelector('#sample')).color") }

    func stylesheetSnapshot() async throws -> String {
        let color = try await self.color()
        let links = try await stylesheetLinks()
        return "color=\(color); links=\(links)"
    }

    private func stylesheetLinks() async throws -> [String] {
        let count = Int(try await page("return String(document.querySelectorAll('link[rel=stylesheet]').length)")) ?? 0
        try NativeUiRuntimeFailure.require(count > 0 && count <= 128, "Diagnostic stylesheet link count exceeded")
        return try await stylesheetDetails(count)
    }

    private func stylesheetDetails(_ count: Int) async throws -> [String] {
        var links: [String] = []
        for position in 0..<count { links.append(try await stylesheetDetail(position)) }
        return links
    }

    private func stylesheetDetail(_ position: Int) async throws -> String {
        let status = try await page("const node=document.querySelectorAll('link[rel=stylesheet]')[\(position)]; return [(new URL(node.href).pathname.startsWith('/.tachyon-native-ui/')?'generation':'root'),node.media,String(node.disabled),String(Boolean(node.sheet))].join('|')")
        let rules = try await page("const node=document.querySelectorAll('link[rel=stylesheet]')[\(position)]; try{return Array.from(node.sheet?.cssRules??[]).map(rule=>rule.cssText).join(';')}catch(error){return String(error)}")
        return "\(position):\(status); rules=\(rules)"
    }

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
