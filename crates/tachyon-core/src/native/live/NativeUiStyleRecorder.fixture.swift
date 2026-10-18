import Foundation
import WebKit

@MainActor final class NativeUiStyleRecorder: NSObject, WKURLSchemeHandler {
    private let scheme: NativeUiStyleScheme
    private(set) var requests: [String] = []
    private(set) var failures: [String] = []

    init(_ assets: NativeUiStyleAssets) { scheme = NativeUiStyleScheme(assets) }

    func webView(_ webView: WKWebView, start task: WKURLSchemeTask) {
        let url = task.request.url
        requests.append(url?.path ?? "")
        if url.flatMap({ try? scheme.assets.asset($0) }) == nil { failures.append(url?.absoluteString ?? "") }
        scheme.webView(webView, start: task)
    }

    func webView(_ webView: WKWebView, stop task: WKURLSchemeTask) {}
}
