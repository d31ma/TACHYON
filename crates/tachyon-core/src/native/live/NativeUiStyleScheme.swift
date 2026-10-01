import Foundation
import WebKit

@MainActor final class NativeUiStyleScheme: NSObject, WKURLSchemeHandler {
    let assets: NativeUiStyleAssets

    init(_ assets: NativeUiStyleAssets) { self.assets = assets }

    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) { deliver(task) }
    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}

    private func deliver(_ task: WKURLSchemeTask) {
        do {
            let (response, bytes) = try response(task)
            task.didReceive(response)
            task.didReceive(bytes)
            task.didFinish()
        } catch { task.didFailWithError(error) }
    }

    private func response(_ task: WKURLSchemeTask) throws -> (URLResponse, Data) {
        let url = try assetURL(task)
        let bytes = try assets.asset(url)
        let fields = ["Content-Type": Self.mime(url.pathExtension), "Content-Length": String(bytes.count), "Cache-Control": "no-store"]
        guard let response = HTTPURLResponse(url: url, statusCode: 200, httpVersion: "HTTP/1.1", headerFields: fields) else {
            throw NativeUiRuntimeFailure(description: "Invalid captured asset response")
        }
        return (response, bytes)
    }

    private func assetURL(_ task: WKURLSchemeTask) throws -> URL {
        guard let url = task.request.url else { throw NativeUiRuntimeFailure(description: "Missing asset URL") }
        return url
    }

    private static func mime(_ suffix: String) -> String {
        switch suffix.lowercased() {
        case "html": return "text/html; charset=utf-8"
        case "css": return "text/css; charset=utf-8"
        case "js", "mjs": return "text/javascript; charset=utf-8"
        case "json": return "application/json"
        case "svg": return "image/svg+xml"
        default: return "application/octet-stream"
        }
    }
}
