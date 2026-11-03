@Setup
final class CatalogSetup {
    final class MemoryCache {
        var entries: [String: BehaviorJSON] = [:]
        let hit = "hit"
    
        func adapter() -> BehaviorCacheAdapter {
            BehaviorCacheAdapter(
                get: { key, _ in
                    self.entries[key] ?? .object([self.hit: .bool(false)])
                },
                set: { key, value, _, expiresAt, _ in
                    let expiry = BehaviorJSON.number(Double(expiresAt))
                    self.entries[key] = .object([self.hit: .bool(true), "value": value, "expiresAt": expiry])
                },
                delete: { key, _ in
                    self.entries.removeValue(forKey: key)
                })
        }
    }

    static let store = MemoryCache()

    static func configure(_ execution: BehaviorExecution) -> BehaviorAdapters {
        var adapters = BehaviorAdapters()
        adapters.caches["catalog"] = store.adapter()
        adapters.authorize = { _, _ in true }
        return adapters
    }
}
