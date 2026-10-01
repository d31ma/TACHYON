import Foundation
import WebKit

@MainActor final class NativeUiStyleHostScheme: NSObject, WKURLSchemeHandler {
    private let assets: NativeUiStyleAssets
    private let document: (String) -> String?

    init(_ assets: NativeUiStyleAssets, _ document: @escaping (String) -> String?) {
        self.assets = assets
        self.document = document
    }

    func webView(_ view: WKWebView, start task: WKURLSchemeTask) { deliver(task) }
    func webView(_ view: WKWebView, stop task: WKURLSchemeTask) {}

    func resolve(_ url: URL) throws -> URL { try resolved(url) }

    private func deliver(_ task: WKURLSchemeTask) {
        do {
            let (response, bytes) = try response(task)
            task.didReceive(response)
            task.didReceive(bytes)
            task.didFinish()
        } catch { task.didFailWithError(error) }
    }

    private func response(_ task: WKURLSchemeTask) throws -> (URLResponse, Data) {
        let url = try requestURL(task)
        let resolved = try resolve(url)
        let bytes = try assets.asset(resolved)
        let fields = ["Content-Type": NativeUiStyleScheme.mime(resolved.pathExtension), "Content-Length": String(bytes.count), "Cache-Control": "no-store"]
        return (try responseURL(url, fields), bytes)
    }

    private func requestURL(_ task: WKURLSchemeTask) throws -> URL {
        guard let url = task.request.url else { throw NativeUiRuntimeFailure(description: "Missing captured host request") }
        return url
    }

    private func responseURL(_ url: URL, _ fields: [String: String]) throws -> URLResponse {
        guard let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: fields) else {
            throw NativeUiRuntimeFailure(description: "Invalid captured host response")
        }
        return response
    }

    private func resolved(_ url: URL) throws -> URL {
        switch url.path.hasPrefix(NativeUiStyleAddress.prefix) {
        case true: _ = try NativeUiStyleAddress.path(url); return url
        case false: return try ordinary(url)
        }
    }

    private func ordinary(_ url: URL) throws -> URL {
        var parts = try components(url)
        parts.query = nil
        parts.fragment = nil
        let normalized = try address(parts)
        try ordinaryPath(normalized)
        let path = document(normalized.path) ?? String(normalized.path.dropFirst())
        parts.path = "/" + capturedPath(path)
        return try address(parts)
    }

    private func ordinaryPath(_ url: URL) throws {
        try NativeUiStyleAddress.origin(url)
        try NativeUiStyleAddress.encodedPath(url)
        try nonRootPath(url)
    }

    private func nonRootPath(_ url: URL) throws {
        switch url.path {
        case "", "/": return
        default: try NativeUiRuntimeAsset.canonicalPath(url.path.trimmingCharacters(in: CharacterSet(charactersIn: "/")))
        }
    }

    private func capturedPath(_ path: String) -> String {
        assets.current.asset(path) != nil ? path : path + "/index.html"
    }

    private func components(_ url: URL) throws -> URLComponents {
        guard let parts = URLComponents(url: url, resolvingAgainstBaseURL: false) else {
            throw NativeUiRuntimeFailure(description: "Invalid captured route URL")
        }
        return parts
    }

    private func address(_ parts: URLComponents) throws -> URL {
        guard let url = parts.url else { throw NativeUiRuntimeFailure(description: "Invalid captured route address") }
        return url
    }
}
