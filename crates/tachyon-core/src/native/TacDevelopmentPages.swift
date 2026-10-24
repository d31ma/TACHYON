private enum TacDevelopmentPages {
    static let origin = "__DEVELOPMENT_ORIGIN__"
    static let limit = 16 * 1024 * 1024
    nonisolated(unsafe) private static var pending: [ObjectIdentifier: URLSessionDataTask] = [:]
    private static let session = URLSession(configuration: .ephemeral)

    static func start(_ task: WKURLSchemeTask, _ path: String, _ policy: String) -> Bool {
        guard let url = task.request.url, var target = URLComponents(string: origin) else { return false }
        target.path = "/" + path
        target.percentEncodedQuery = URLComponents(url: url, resolvingAgainstBaseURL: false)?.percentEncodedQuery
        guard let address = target.url else { return false }
        let key = ObjectIdentifier(task as AnyObject)
        let request = URLRequest(url: address, cachePolicy: .reloadIgnoringLocalCacheData, timeoutInterval: 30)
        let fetch = session.dataTask(with: request) { data, response, error in
            DispatchQueue.main.async {
                guard pending.removeValue(forKey: key) != nil else { return }
                answer(task, url, policy, error == nil ? data : nil, response as? HTTPURLResponse)
            }
        }
        pending[key] = fetch
        fetch.resume()
        return true
    }

    static func stop(_ task: WKURLSchemeTask) {
        pending.removeValue(forKey: ObjectIdentifier(task as AnyObject))?.cancel()
    }

    private static func answer(_ task: WKURLSchemeTask, _ url: URL, _ policy: String, _ data: Data?, _ response: HTTPURLResponse?) {
        guard let data, data.count <= limit, let response, let reply = HTTPURLResponse(
            url: url,
            statusCode: response.statusCode,
            httpVersion: "HTTP/1.1",
            headerFields: [
                "Content-Type": response.value(forHTTPHeaderField: "Content-Type") ?? "application/octet-stream",
                "Content-Length": String(data.count),
                "Content-Security-Policy": policy,
                "Cache-Control": "no-store",
                "X-Content-Type-Options": "nosniff",
            ]
        ) else {
            task.didFailWithError(NSError(domain: "TachyonDevelopment", code: 1))
            return
        }
        task.didReceive(reply)
        task.didReceive(data)
        task.didFinish()
    }
}
