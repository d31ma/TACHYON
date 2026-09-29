@_extern(wasm, module: "tachyon", name: "tac_host_epoch_ms")
func tacHostEpochMilliseconds() -> Double

@_extern(wasm, module: "tachyon", name: "tac_host_monotonic_ms")
func tacHostMonotonicMilliseconds() -> Double

@_extern(wasm, module: "tachyon", name: "tac_host_web")
func tacHostWeb(_ pointer: UnsafePointer<UInt8>?, _ length: Int32) -> UnsafeMutablePointer<CChar>?

enum TacWebHost {
    static let noPage = "This browser API needs the page, and this service runs without one."
    static let wrongAnswer = "The page gave a browser API answer of the wrong type."

    static func epochMilliseconds() -> Int64 {
        Int64(tacHostEpochMilliseconds())
    }

    static func call(_ api: String, _ call: String, _ arguments: [Any?] = []) throws -> Any? {
        let request = "{\"api\":\(tacEncode(api)),\"call\":\(tacEncode(call)),\"args\":\(tacEncode(arguments))}"
        guard let answer = tacParseJSON(ask(request)) as? [String: Any] else { throw TacBrowserError(noPage) }
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

    private static func ask(_ request: String) -> String {
        var text = request
        guard let answer = text.withUTF8({ tacHostWeb($0.baseAddress, Int32($0.count)) }) else { return "" }
        defer { answer.deallocate() }
        return String(cString: answer)
    }
}
