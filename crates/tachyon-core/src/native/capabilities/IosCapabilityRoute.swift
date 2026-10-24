private enum IosCapabilityRoute {
    static let path = "/__tachyon/capability/"
    nonisolated(unsafe) private static var pending: Set<ObjectIdentifier> = []

    static func start(_ webView: WKWebView, _ task: WKURLSchemeTask) -> Bool {
        guard let url = task.request.url, url.path.hasPrefix(path) else { return false }
        let capability = String(url.path.dropFirst(path.count))
        guard task.request.httpMethod == "POST", capability.hasPrefix("filePicker."), capability.utf8.count <= 64,
              task.request.value(forHTTPHeaderField: "X-Tachyon-Capability") == capability,
              task.request.mainDocumentURL.map({ $0.scheme == "tachyon-app" && $0.host == "bundle" }) ?? true,
              let body = task.request.httpBody, body.count <= 65536,
              let request = String(data: body, encoding: .utf8).flatMap(tachyonParseJSONRequest),
              let payload = tachyonCanonicalJSONRequest(request) else {
            task.didFailWithError(NSError(domain: "TachyonCapability", code: 1))
            return true
        }
        let key = ObjectIdentifier(task as AnyObject)
        pending.insert(key)
        DispatchQueue.main.async {
            MainActor.assumeIsolated {
                let answered: (String) -> Void = { answer in
                    guard pending.remove(key) != nil else { return }
                    respond(task, answer)
                }
                guard NativeBridge.shared.webView === webView, let request = tachyonParseJSONRequest(payload) else {
                    answered("{\"status\":\"unavailable\"}")
                    return
                }
                IosFilePicker.shared.call(String(capability.dropFirst(11)), request, answered)
            }
        }
        return true
    }

    static func stop(_ task: WKURLSchemeTask) {
        pending.remove(ObjectIdentifier(task as AnyObject))
    }

    private static func respond(_ task: WKURLSchemeTask, _ answer: String) {
        let data = Data(answer.utf8)
        guard let url = task.request.url, let response = HTTPURLResponse(
            url: url,
            statusCode: 200,
            httpVersion: "HTTP/1.1",
            headerFields: [
                "Content-Type": "application/json",
                "Content-Length": String(data.count),
                "Cache-Control": "no-store",
                "X-Content-Type-Options": "nosniff",
            ]
        ) else {
            task.didFailWithError(NSError(domain: "TachyonCapability", code: 2))
            return
        }
        task.didReceive(response)
        task.didReceive(data)
        task.didFinish()
    }
}
