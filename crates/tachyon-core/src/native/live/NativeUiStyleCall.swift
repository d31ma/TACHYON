import Foundation
import WebKit

@MainActor struct NativeUiStyleCall {
    static let world = WKContentWorld.world(name: "TachyonNativeStylesV1")

    static func run(_ view: WKWebView, _ operation: String, _ ticket: NativeUiStyleTicket, _ styles: [String: [String: String]] = [:]) async throws -> String {
        let arguments = try checked(operation, ticket, styles)
        return try await execute(view, arguments)
    }

    static func checked(_ operation: String, _ ticket: NativeUiStyleTicket, _ styles: [String: [String: String]]) throws -> [String: Any] {
        let arguments: [String: Any] = ["operation": operation, "ticket": ticket.arguments(), "styles": styles, "deadline": 10000]
        try bounded(arguments)
        return arguments
    }

    static func finalize(_ view: WKWebView, _ previous: NativeUiStyleTicket?) async throws { try await finalized(view, previous) }

    private static func finalized(_ view: WKWebView, _ previous: NativeUiStyleTicket?) async throws {
        guard let previous else { return }
        let reply = try await run(view, "finalize", previous)
        try NativeUiRuntimeFailure.require(reply == "finalized", "Prior styles could not finalize")
    }

    private static func bounded(_ arguments: [String: Any]) throws {
        let bytes = try JSONSerialization.data(withJSONObject: arguments)
        try NativeUiRuntimeFailure.require(bytes.count <= 65536, "Stylesheet native metadata limit exceeded")
    }

    private static func execute(_ view: WKWebView, _ arguments: [String: Any]) async throws -> String {
        try await withCheckedThrowingContinuation { continuation in
            let reply = NativeUiStyleReply(continuation)
            reply.deadline()
            view.callAsyncJavaScript(NativeUiStyleScript.source, arguments: arguments, in: nil, contentWorld: world) { result in
                reply.resolve(result)
            }
        }
    }
}
