import Foundation
import WebKit

@MainActor final class NativeUiHostFrameFixture: NSObject, WKScriptMessageHandler {
    private static let name = "nativeStylesFrameFixture"
    private var frame: WKFrameInfo?

    func configure(_ configuration: WKWebViewConfiguration) {
        configuration.userContentController.addScriptMessageHandler(self, contentWorld: NativeUiStyleCall.world, name: Self.name)
        let source = "if(window.top!==window)window.webkit.messageHandlers.nativeStylesFrameFixture.postMessage('ready')"
        configuration.userContentController.addUserScript(WKUserScript(source: source, injectionTime: .atDocumentEnd, forMainFrameOnly: false, in: NativeUiStyleCall.world))
    }

    func userContentController(_ controller: WKUserContentController, didReceive message: WKScriptMessage) {
        received(message)
    }

    func check(_ view: WKWebView, _ candidate: NativeUiRuntimeGeneration, _ assets: NativeUiStyleAssets) async throws {
        let frame = try await ready()
        let ticket = try childTicket(frame, candidate)
        let styles = try assets.stylesheetArguments(candidate)
        let arguments = try NativeUiStyleCall.checked("prepare", ticket, styles)
        let refusal = try await refused(view, frame, arguments)
        try NativeUiRuntimeFailure.require(refusal == "Stylesheet selection requires the owned main frame", "Child frame was not refused by the real private-world transaction")
    }

    private func received(_ message: WKScriptMessage) {
        guard !message.frameInfo.isMainFrame else { return }
        frame = message.frameInfo
    }

    private func ready() async throws -> WKFrameInfo {
        let deadline = Date().addingTimeInterval(10)
        while frame == nil {
            try NativeUiRuntimeFailure.require(Date() < deadline, "Private-world child frame did not become ready")
            try await Task.sleep(nanoseconds: 10_000_000)
        }
        return frame!
    }

    private func childTicket(_ frame: WKFrameInfo, _ candidate: NativeUiRuntimeGeneration) throws -> NativeUiStyleTicket {
        guard let url = frame.request.url else { throw NativeUiRuntimeFailure(description: "Child frame has no actual URL") }
        try NativeUiRuntimeFailure.require(url.absoluteString == "tachyon-app://bundle/index.html?child=1", "Unexpected child frame document")
        return NativeUiStyleTicket(token: UUID().uuidString, epoch: 0, url: url.absoluteString, generation: candidate.descriptor.generation)
    }

    private func refused(_ view: WKWebView, _ frame: WKFrameInfo, _ arguments: [String: Any]) async throws -> String {
        let body = "try { return await (async()=>{\n" + NativeUiStyleScript.source + "\n})(); } catch(error) { return String(error.message); }"
        return try await withCheckedThrowingContinuation { continuation in
            let reply = NativeUiStyleReply(continuation)
            reply.deadline()
            view.callAsyncJavaScript(body, arguments: arguments, in: frame, in: NativeUiStyleCall.world, completionHandler: { reply.resolve($0) })
        }
    }
}
