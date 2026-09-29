struct TacListValueEncoder: UnkeyedEncodingContainer {
    let node: TacEncodedNode
    let codingPath: [CodingKey]

    init(_ node: TacEncodedNode, _ codingPath: [CodingKey]) {
        self.node = node
        self.codingPath = codingPath
    }

    var count: Int { node.itemCount }

    mutating func encodeNil() throws {
        node.item().value = nil
    }

    mutating func encode<T: Encodable>(_ value: T) throws {
        var single = TacSingleValueEncoder(node.item(), codingPath)
        try single.encode(value)
    }

    mutating func nestedContainer<NestedKey: CodingKey>(
        keyedBy keyType: NestedKey.Type
    ) -> KeyedEncodingContainer<NestedKey> {
        TacValueEncoder(node.item(), codingPath).container(keyedBy: keyType)
    }

    mutating func nestedUnkeyedContainer() -> UnkeyedEncodingContainer {
        TacValueEncoder(node.item(), codingPath).unkeyedContainer()
    }

    mutating func superEncoder() -> Encoder {
        TacValueEncoder(node.item(), codingPath)
    }
}
