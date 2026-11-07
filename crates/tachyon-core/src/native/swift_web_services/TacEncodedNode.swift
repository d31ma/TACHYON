final class TacEncodedNode {
    var value: Any?
    private var fields: [String: TacEncodedNode]?
    private var items: [TacEncodedNode]?

    var plain: Any? {
        if let fields { return fields.mapValues { $0.plain } as [String: Any?] }
        if let items { return items.map { $0.plain } as [Any?] }
        return value
    }

    var itemCount: Int { items?.count ?? 0 }

    func field(_ key: String) -> TacEncodedNode {
        let child = TacEncodedNode()
        fields = (fields ?? [:]).merging([key: child]) { _, added in added }
        return child
    }

    func item() -> TacEncodedNode {
        let child = TacEncodedNode()
        items = (items ?? []) + [child]
        return child
    }

    func becomeObject() {
        fields = fields ?? [:]
    }

    func becomeList() {
        items = items ?? []
    }
}
