enum BehaviorPlatform {
    static var now: Int64 { Int64(Foundation.Date().timeIntervalSince1970 * 1000) }

    static func quoted(_ value: String) throws -> String {
        let data = try Foundation.JSONSerialization.data(withJSONObject: value, options: [.fragmentsAllowed, .withoutEscapingSlashes])
        guard let text = String(data: data, encoding: .utf8) else { throw BehaviorError("JSON_STRING") }
        return text
    }

    static func number(_ bytes: ArraySlice<UInt8>) throws -> Double {
        let value = try Foundation.JSONSerialization.jsonObject(with: Foundation.Data(bytes), options: [.fragmentsAllowed])
        guard let number = value as? Foundation.NSNumber else { throw BehaviorError("JSON_INVALID") }
        return number.doubleValue
    }

    static func text(_ bytes: ArraySlice<UInt8>) throws -> String {
        let value = try Foundation.JSONSerialization.jsonObject(with: Foundation.Data(bytes), options: [.fragmentsAllowed])
        guard let text = value as? String else { throw BehaviorError("JSON_STRING") }
        return text
    }

    static func matches(_ value: String, _ pattern: String) -> Bool {
        value.range(of: pattern, options: .regularExpression) == value.startIndex..<value.endIndex
    }

    static func replaced(_ text: String, _ target: String, _ replacement: String) -> String {
        text.replacingOccurrences(of: target, with: replacement)
    }

    static func power(_ base: Double, _ exponent: Double) -> Double {
        pow(base, exponent)
    }

    static func builtinCache(_ name: String) -> BehaviorCacheAdapter? {
        nil
    }

    static func builtinEntities(_ name: String) -> BehaviorEntityAdapter? {
        nil
    }
}
