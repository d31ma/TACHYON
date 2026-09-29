enum TacWebHost {
    static let noPage = "This browser API needs the page, and this service runs without one."
    static let wrongAnswer = "The page gave a browser API answer of the wrong type."

    static func call(_ api: String, _ call: String, _ arguments: [Any?] = []) throws -> Any? {
        guard let answer = TacPage.answer(api, call, arguments) else { throw TacBrowserError(noPage) }
        if let error = answer["error"] as? String { throw TacBrowserError(error) }
        return answer["value"]
    }

    static func text(_ api: String, _ call: String) throws -> String {
        try answer(Self.call(api, call), as: String.self)
    }

    static func flag(_ api: String, _ call: String) throws -> Bool {
        try answer(Self.call(api, call), as: Bool.self)
    }

    static func answer<Value>(_ value: Any?, as type: Value.Type) throws -> Value {
        guard let typed = value as? Value else { throw TacBrowserError(wrongAnswer) }
        return typed
    }
}
