final class TacRustHttp: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    static let send: @convention(c) (UInt64, UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<CChar>?, UnsafePointer<UInt8>?, Int, UInt64, Int) -> Void = {
        ticket, method, address, headers, body, length, timeout, limit in
        let call = TacRustHttp(ticket: ticket, limit: limit)
        guard let request = TacRustHttp.request(method, address, headers, body, length, timeout) else { return call.finish(0, "HTTP_REQUEST") }
        call.start(request, timeout)
    }

    private let ticket: UInt64
    private let limit: Int
    private let lock = NSLock()
    private var received = Foundation.Data()
    private var answered = false
    private var expired = false

    private init(ticket: UInt64, limit: Int) {
        self.ticket = ticket
        self.limit = limit
    }

    private static func request(
        _ method: UnsafePointer<CChar>?, _ address: UnsafePointer<CChar>?, _ headers: UnsafePointer<CChar>?,
        _ body: UnsafePointer<UInt8>?, _ length: Int, _ timeout: UInt64
    ) -> URLRequest? {
        guard let method, let address, let url = URL(string: String(cString: address)) else { return nil }
        var request = URLRequest(url: url)
        request.httpMethod = String(cString: method)
        request.httpShouldHandleCookies = false
        request.cachePolicy = .reloadIgnoringLocalCacheData
        request.timeoutInterval = TimeInterval(max(timeout, 1)) / 1000
        let lines = headers.map { String(cString: $0) }?.components(separatedBy: "\n") ?? []
        for index in stride(from: 0, to: lines.count - 1, by: 2) {
            request.addValue(lines[index + 1], forHTTPHeaderField: lines[index])
        }
        if let body { request.httpBody = Foundation.Data(bytes: body, count: length) }
        return request
    }

    private func start(_ request: URLRequest, _ timeout: UInt64) {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.httpCookieStorage = nil
        configuration.httpShouldSetCookies = false
        configuration.urlCache = nil
        let session = URLSession(configuration: configuration, delegate: self, delegateQueue: nil)
        let task = session.dataTask(with: request)
        DispatchQueue.global().asyncAfter(deadline: .now() + .milliseconds(Int(min(max(timeout, 1), 600_000)))) {
            self.lock.lock()
            self.expired = true
            self.lock.unlock()
            task.cancel()
        }
        task.resume()
        session.finishTasksAndInvalidate()
    }

    private func finish(_ status: UInt32, _ error: String?) {
        lock.lock()
        let first = !answered
        answered = true
        let bytes = received
        lock.unlock()
        guard first else { return }
        let answer = { (pointer: UnsafePointer<CChar>?) in
            bytes.withUnsafeBytes { tacRustNativeHttpAnswer(self.ticket, status, $0.bindMemory(to: UInt8.self).baseAddress, bytes.count, pointer) }
        }
        guard let error else { return answer(nil) }
        error.withCString { answer($0) }
    }

    private func failure(_ error: Error) -> String {
        lock.lock()
        let late = expired
        lock.unlock()
        return late || (error as? URLError)?.code == .timedOut ? "DEADLINE" : "HTTP_NETWORK"
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive response: URLResponse, completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        let status = UInt32((response as? HTTPURLResponse)?.statusCode ?? 0)
        guard (200...299).contains(status) else {
            finish(status, nil)
            return completionHandler(.cancel)
        }
        guard response.expectedContentLength <= Int64(limit) else {
            finish(0, "HTTP_LIMIT")
            return completionHandler(.cancel)
        }
        completionHandler(.allow)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive data: Data) {
        lock.lock()
        received.append(data)
        let over = received.count > limit
        lock.unlock()
        guard over else { return }
        finish(0, "HTTP_LIMIT")
        dataTask.cancel()
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        completionHandler(nil)
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let error else {
            return finish(UInt32((task.response as? HTTPURLResponse)?.statusCode ?? 0), nil)
        }
        finish(0, failure(error))
    }
}
