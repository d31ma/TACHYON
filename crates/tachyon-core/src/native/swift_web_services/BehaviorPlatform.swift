enum BehaviorPlatform {
    static let browserEntities = "indexeddb"

    static var now: Int64 { Int64(TacPage.epochMilliseconds()) }

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
        BehaviorTextPattern.matches(value, pattern)
    }

    static func replaced(_ text: String, _ target: String, _ replacement: String) -> String {
        guard !target.isEmpty else { return text }
        var result = ""
        var rest = Substring(text)
        while !rest.isEmpty {
            if rest.hasPrefix(target) {
                result += replacement
                rest = rest.dropFirst(target.count)
            } else {
                result.append(rest.removeFirst())
            }
        }
        return result
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

    static func exchange(_ request: BehaviorJSON) async throws -> BehaviorJSON {
        try await BehaviorBrowserHttp.send(request)
    }

    static func model<T: Decodable>(_ type: T.Type, _ value: BehaviorJSON) throws -> T {
        try TacValueDecoder.model(type, tacParseJSON(value.encoded()))
    }
}
