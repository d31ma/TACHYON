enum BehaviorBrowserStorage {
    static func cacheEntry(_ name: String, _ key: String) throws -> BehaviorJSON {
        try ask(["op": .string("cache.get"), "cache": .string(name), "key": .string(key)])
    }

    static func saveCacheEntry(
        _ name: String, _ key: String, _ value: BehaviorJSON, _ policy: BehaviorJSON, _ expiresAt: Int64
    ) throws {
        _ = try ask([
            "op": .string("cache.set"), "cache": .string(name), "key": .string(key), "value": value,
            "expiresAt": .number(Double(expiresAt)), "maxEntries": policy.get("maxEntries"),
        ])
    }

    static func removeCacheEntry(_ name: String, _ key: String) throws {
        _ = try ask(["op": .string("cache.delete"), "cache": .string(name), "key": .string(key)])
    }

    static func entityCommand(_ command: BehaviorJSON) throws -> BehaviorJSON {
        try ask(["op": .string("entity"), "command": command])
    }

    private static func ask(_ request: [String: BehaviorJSON]) throws -> BehaviorJSON {
        guard let text = TacWebStorage.ask(try BehaviorJSON.object(request).encoded()) else {
            throw BehaviorError("STORAGE_MISSING")
        }
        let answer = try BehaviorJSON.parse(text)
        guard !answer.has("error") else { throw BehaviorError("STORAGE_REFUSED") }
        return answer.get("value")
    }
}
