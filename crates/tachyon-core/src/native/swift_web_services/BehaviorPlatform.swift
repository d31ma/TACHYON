enum BehaviorPlatform {
    static let browserEntities = "indexeddb"

    static var now: Int64 { TacWebHost.epochMilliseconds() }

    static func quoted(_ value: String) throws -> String {
        tacEncode(value)
    }

    static func number(_ bytes: ArraySlice<UInt8>) throws -> Double {
        let found = tacParseJSON(String(decoding: bytes, as: UTF8.self))
        if let whole = found as? Int { return Double(whole) }
        guard let real = found as? Double else { throw BehaviorError("JSON_INVALID") }
        return real
    }

    static func text(_ bytes: ArraySlice<UInt8>) throws -> String {
        guard let text = tacParseJSON(String(decoding: bytes, as: UTF8.self)) as? String else {
            throw BehaviorError("JSON_STRING")
        }
        return text
    }

    static func matches(_ value: String, _ pattern: String) -> Bool {
        guard let expression = try? Regex(pattern) else { return false }
        return value.wholeMatch(of: expression) != nil
    }

    static func replaced(_ text: String, _ target: String, _ replacement: String) -> String {
        text.replacing(target, with: replacement)
    }

    static func power(_ base: Double, _ exponent: Double) -> Double {
        var result = 1.0
        for _ in 0..<Int(exponent) { result *= base }
        return result
    }

    static func builtinCache(_ name: String) -> BehaviorCacheAdapter? {
        BehaviorCacheAdapter(
            get: { key, _ in try BehaviorBrowserStorage.cacheEntry(name, key) },
            set: { key, value, policy, expiresAt, _ in
                try BehaviorBrowserStorage.saveCacheEntry(name, key, value, policy, expiresAt)
            },
            delete: { key, _ in try BehaviorBrowserStorage.removeCacheEntry(name, key) })
    }

    static func builtinEntities(_ name: String) -> BehaviorEntityAdapter? {
        guard name == browserEntities else { return nil }
        return BehaviorEntityAdapter { command, _ in try BehaviorBrowserStorage.entityCommand(command) }
    }
}
