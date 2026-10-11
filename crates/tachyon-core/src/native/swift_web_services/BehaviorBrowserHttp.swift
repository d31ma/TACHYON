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
        if error != .null { throw BehaviorError(code(error.get("code"))) }
        guard answer.has("value") else { throw BehaviorError(wrongAnswer) }
        return answer.get("value")
    }

    private static func parsed(_ text: String) throws -> BehaviorJSON {
        guard let answer = try? BehaviorJSON.parse(text), case .object = answer else { throw BehaviorError(wrongAnswer) }
        return answer
    }

    private static func code(_ named: BehaviorJSON) -> String {
        guard case .string(let code) = named, codes.contains(code) else { return network }
        return code
    }
}
