enum BrowserFakes {
    static let missing = "This browser API needs the page. A test unit has no page: give the call an answer with BrowserFakes.answer in the test unit."
    nonisolated(unsafe) private static var answers: [String: Any] = [:]

    static func answer(_ api: String, _ call: String, _ value: Any?) {
        answers[api + "." + call] = value ?? NSNull()
    }

    static func reply(_ api: String, _ call: String) -> [String: Any] {
        guard let value = answers[api + "." + call] else { return ["error": missing] }
        return ["value": asThePageSendsIt(value)]
    }

    private static func asThePageSendsIt(_ value: Any) -> Any {
        guard let text = try? JSONSerialization.data(withJSONObject: value, options: [.fragmentsAllowed]),
              let sent = try? JSONSerialization.jsonObject(with: text, options: [.fragmentsAllowed])
        else { return value }
        return sent
    }
}
