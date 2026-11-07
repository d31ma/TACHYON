final class BehaviorYonHttp: NSObject, URLSessionDataDelegate, @unchecked Sendable {
    static let origin = "__TAC_YON_ORIGIN__"
    static let methods: Set<String> = ["GET", "HEAD", "POST", "PUT", "PATCH", "DELETE"]
    static let refusedHeaders: Set<String> = ["host", "connection", "content-length", "transfer-encoding", "cookie", "origin", "referer"]
    static let refusedHeaderPrefixes = ["sec-", "proxy-"]
    static let queryCharacters = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")
    static let mostResponseBytes = 1_048_576
    static let mostWaitMilliseconds: Int64 = 60_000
    static let mostPathBytes = 2048
    static let mostAddressBytes = 8192

    private let limit: Int
    private let lock = NSLock()
    private var received = Foundation.Data()
    private var continuation: CheckedContinuation<Data, Error>?

    private init(limit: Int) {
        self.limit = limit
    }

    static func send(_ request: BehaviorJSON) async throws -> BehaviorJSON {
        let limit = try request.get("maxResponseBytes").integer
        guard (1...mostResponseBytes).contains(limit) else { throw BehaviorError("HTTP_LIMIT") }
        let prepared = try urlRequest(request)
        return try answer(await BehaviorYonHttp(limit: limit).run(prepared))
    }

    private static func urlRequest(_ request: BehaviorJSON) throws -> URLRequest {
        let method = try request.get("method").text
        guard methods.contains(method), try request.get("redirect").text == "error", try request.get("credentials").text == "omit" else { throw BehaviorError("HTTP_REQUEST") }
        var prepared = URLRequest(url: try address(request))
        prepared.httpMethod = method
        prepared.httpShouldHandleCookies = false
        prepared.cachePolicy = .reloadIgnoringLocalCacheData
        prepared.timeoutInterval = try seconds(request.get("deadline").milliseconds)
        for (name, value) in try request.get("headers").members {
            prepared.setValue(try header(name, value), forHTTPHeaderField: name)
        }
        guard request.has("body") else { return prepared }
        guard !["GET", "HEAD"].contains(method) else { throw BehaviorError("HTTP_BODY") }
        prepared.httpBody = Foundation.Data(try request.get("body").encoded().utf8)
        prepared.setValue("application/json", forHTTPHeaderField: "Content-Type")
        return prepared
    }

    private static func seconds(_ deadline: Int64) throws -> TimeInterval {
        let remaining = deadline - Int64(Foundation.Date().timeIntervalSince1970 * 1000)
        guard remaining > 0 else { throw BehaviorError("DEADLINE") }
        guard remaining <= mostWaitMilliseconds else { throw BehaviorError("HTTP_REQUEST") }
        return TimeInterval(remaining) / 1000
    }

    private static func address(_ request: BehaviorJSON) throws -> URL {
        let path = try request.get("path").text
        guard safePath(path) else { throw BehaviorError("HTTP_ORIGIN") }
        let query = try request.get("query").members.sorted { $0.key < $1.key }.map { try queryPair($0.key, $0.value) }
        let text = origin + path + (query.isEmpty ? "" : "?" + query.joined(separator: "&"))
        guard text.utf8.count <= mostAddressBytes, let url = URL(string: text) else { throw BehaviorError("HTTP_QUERY") }
        return url
    }

    private static func safePath(_ path: String) -> Bool {
        guard path.hasPrefix("/"), !path.hasPrefix("//"), path.utf8.count <= mostPathBytes else { return false }
        if path.unicodeScalars.contains(where: { $0.value <= 32 || "\\?#%".unicodeScalars.contains($0) }) { return false }
        return !path.split(separator: "/", omittingEmptySubsequences: false).contains { $0 == "." || $0 == ".." }
    }

    private static func queryPair(_ name: String, _ value: BehaviorJSON) throws -> String {
        let text: String
        switch value {
        case .string(let written): text = written
        case .number, .bool: text = try value.encoded()
        default: throw BehaviorError("HTTP_QUERY")
        }
        guard let encodedName = name.addingPercentEncoding(withAllowedCharacters: queryCharacters),
              let encodedText = text.addingPercentEncoding(withAllowedCharacters: queryCharacters) else { throw BehaviorError("HTTP_QUERY") }
        return encodedName + "=" + encodedText
    }

    private static func header(_ name: String, _ value: BehaviorJSON) throws -> String {
        let lowered = name.lowercased()
        guard !refusedHeaders.contains(lowered), !refusedHeaderPrefixes.contains(where: { lowered.hasPrefix($0) }) else { throw BehaviorError("HTTP_HEADER") }
        return try value.text
    }

    private static func answer(_ bytes: Data) throws -> BehaviorJSON {
        guard let text = String(data: bytes, encoding: .utf8) else { throw BehaviorError("HTTP_RESPONSE") }
        if text.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty { return .null }
        guard let value = try? BehaviorJSON.parse(text) else { throw BehaviorError("HTTP_RESPONSE") }
        return value
    }

    private static func failure(_ error: Error) -> BehaviorError {
        guard let failed = error as? URLError else { return BehaviorError("HTTP_NETWORK") }
        switch failed.code {
        case .timedOut: return BehaviorError("DEADLINE")
        case .cancelled: return BehaviorError("CANCELLED")
        default: return BehaviorError("HTTP_NETWORK")
        }
    }

    private func run(_ request: URLRequest) async throws -> Data {
        let configuration = URLSessionConfiguration.ephemeral
        configuration.httpCookieStorage = nil
        configuration.httpShouldSetCookies = false
        configuration.urlCache = nil
        let session = URLSession(configuration: configuration, delegate: self, delegateQueue: nil)
        defer { session.finishTasksAndInvalidate() }
        let task = session.dataTask(with: request)
        return try await withTaskCancellationHandler {
            try await withCheckedThrowingContinuation { (waiting: CheckedContinuation<Data, Error>) in
                lock.lock()
                continuation = waiting
                lock.unlock()
                task.resume()
            }
        } onCancel: {
            task.cancel()
        }
    }

    private func finish(_ result: Result<Data, Error>) {
        lock.lock()
        let waiting = continuation
        continuation = nil
        lock.unlock()
        waiting?.resume(with: result)
    }

    func urlSession(_ session: URLSession, dataTask: URLSessionDataTask, didReceive response: URLResponse, completionHandler: @escaping (URLSession.ResponseDisposition) -> Void) {
        let status = (response as? HTTPURLResponse)?.statusCode ?? 0
        guard (200...299).contains(status) else {
            finish(.failure(BehaviorError("HTTP_STATUS")))
            return completionHandler(.cancel)
        }
        guard response.expectedContentLength <= Int64(limit) else {
            finish(.failure(BehaviorError("HTTP_LIMIT")))
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
        finish(.failure(BehaviorError("HTTP_LIMIT")))
        dataTask.cancel()
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, willPerformHTTPRedirection response: HTTPURLResponse, newRequest request: URLRequest, completionHandler: @escaping (URLRequest?) -> Void) {
        finish(.failure(BehaviorError("HTTP_NETWORK")))
        completionHandler(nil)
        task.cancel()
    }

    func urlSession(_ session: URLSession, task: URLSessionTask, didCompleteWithError error: Error?) {
        guard let error else {
            lock.lock()
            let bytes = received
            lock.unlock()
            return finish(.success(bytes))
        }
        finish(.failure(Self.failure(error)))
    }
}
