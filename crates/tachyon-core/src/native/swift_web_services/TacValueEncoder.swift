struct TacValueEncoder: Encoder {
    let node: TacEncodedNode
    let codingPath: [CodingKey]
    let userInfo: [CodingUserInfoKey: Any] = [:]

    init(_ node: TacEncodedNode, _ codingPath: [CodingKey] = []) {
        self.node = node
        self.codingPath = codingPath
    }

    static func plain<T: Encodable>(_ value: T) throws -> Any? {
        let root = TacEncodedNode()
        var single = TacSingleValueEncoder(root, [])
        try single.encode(value)
        return root.plain
    }

    func container<Key: CodingKey>(keyedBy type: Key.Type) -> KeyedEncodingContainer<Key> {
        node.becomeObject()
        return KeyedEncodingContainer(TacKeyedValueEncoder<Key>(node, codingPath))
    }

    func unkeyedContainer() -> UnkeyedEncodingContainer {
        node.becomeList()
        return TacListValueEncoder(node, codingPath)
    }

    func singleValueContainer() -> SingleValueEncodingContainer {
        TacSingleValueEncoder(node, codingPath)
    }
}
