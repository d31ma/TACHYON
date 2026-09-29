@_extern(wasm, module: "tachyon", name: "tac_host_epoch_ms")
func tacHostEpochMilliseconds() -> Double

@_extern(wasm, module: "tachyon", name: "tac_host_monotonic_ms")
func tacHostMonotonicMilliseconds() -> Double

@_extern(wasm, module: "tachyon", name: "tac_host_web")
func tacHostWeb(_ pointer: UnsafePointer<UInt8>?, _ length: Int32) -> UnsafeMutablePointer<CChar>?

enum TacPage {
    static func epochMilliseconds() -> Double {
        tacHostEpochMilliseconds()
    }

    static func monotonicMilliseconds() -> Double {
        tacHostMonotonicMilliseconds()
    }

    static func answer(_ api: String, _ call: String, _ arguments: [Any?]) -> [String: Any]? {
        let request = "{\"api\":\(tacEncode(api)),\"call\":\(tacEncode(call)),\"args\":\(tacEncode(arguments))}"
        return tacParseJSON(ask(request)) as? [String: Any]
    }

    private static func ask(_ request: String) -> String {
        var text = request
        guard let answer = text.withUTF8({ tacHostWeb($0.baseAddress, Int32($0.count)) }) else { return "" }
        defer { answer.deallocate() }
        return String(cString: answer)
    }
}
