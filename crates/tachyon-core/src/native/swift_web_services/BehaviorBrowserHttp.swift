enum BehaviorBrowserHttp {
    static let requestBytes = 1048576
    static let network = "HTTP_NETWORK"
    static let wrongAnswer = "HTTP_RESPONSE"
    static let codes: Set<String> = [
        "HTTP_REQUEST", "HTTP_ORIGIN", "HTTP_HEADER", "HTTP_QUERY", "HTTP_BODY", "HTTP_LIMIT", "HTTP_RESPONSE",
        "HTTP_STATUS", "HTTP_CAPACITY", "HTTP_NETWORK", "HTTP_MISSING", "CANCELLED", "DEADLINE",
    ]

    static func send(_ request: BehaviorJSON) async throws -> BehaviorJSON {
        let text = try request.encoded(maximum: requestBytes)
        let answer = try parsed(await TacHttpTickets.exchange(text))
        let error = answer.get("error")
        if error != .null { throw failure(error) }
        guard answer.has("value") else { throw BehaviorError(wrongAnswer) }
        return answer.get("value")
    }

    private static func parsed(_ text: String) throws -> BehaviorJSON {
        guard let answer = try? BehaviorJSON.parse(text), case .object = answer else { throw BehaviorError(wrongAnswer) }
        return answer
    }

    private static func failure(_ error: BehaviorJSON) -> BehaviorError {
        BehaviorError(code(error.get("code")), message: try? error.get("message").text, status: status(error.get("status")), body: body(error))
    }

    private static func status(_ number: BehaviorJSON) -> Int {
        guard let whole = try? number.integer else { return 0 }
        return whole
    }

    private static func body(_ error: BehaviorJSON) -> BehaviorJSON? {
        guard error.has("body") else { return nil }
        return error.get("body")
    }

    private static func code(_ named: BehaviorJSON) -> String {
        guard case .string(let code) = named, codes.contains(code) else { return network }
        return code
    }
}
