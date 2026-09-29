struct TacKeyedValueEncoder<Key: CodingKey>: KeyedEncodingContainerProtocol {
    let node: TacEncodedNode
    let codingPath: [CodingKey]

    init(_ node: TacEncodedNode, _ codingPath: [CodingKey]) {
        self.node = node
        self.codingPath = codingPath
    }

    mutating func encodeNil(forKey key: Key) throws {
        node.field(key.stringValue).value = nil
    }

    mutating func encode<T: Encodable>(_ value: T, forKey key: Key) throws {
        var single = TacSingleValueEncoder(node.field(key.stringValue), codingPath + [key])
        try single.encode(value)
    }

    mutating func nestedContainer<NestedKey: CodingKey>(
        keyedBy keyType: NestedKey.Type, forKey key: Key
    ) -> KeyedEncodingContainer<NestedKey> {
        encoder(key).container(keyedBy: keyType)
    }

    mutating func nestedUnkeyedContainer(forKey key: Key) -> UnkeyedEncodingContainer {
        encoder(key).unkeyedContainer()
    }

    mutating func superEncoder() -> Encoder {
        TacValueEncoder(node, codingPath)
    }

    mutating func superEncoder(forKey key: Key) -> Encoder {
        encoder(key)
    }

    private func encoder(_ key: Key) -> TacValueEncoder {
        TacValueEncoder(node.field(key.stringValue), codingPath + [key])
    }
}
