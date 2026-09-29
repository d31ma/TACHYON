struct TacValueDecoder: Decoder {
    let value: Any?
    let codingPath: [CodingKey]
    let userInfo: [CodingUserInfoKey: Any] = [:]

    init(_ value: Any?, _ codingPath: [CodingKey] = []) {
        self.value = value
        self.codingPath = codingPath
    }

    static func model<T: Decodable>(_ type: T.Type, _ value: Any?) throws -> T {
        try TacSingleValueDecoder(value, []).decode(type)
    }

    func container<Key: CodingKey>(keyedBy type: Key.Type) throws -> KeyedDecodingContainer<Key> {
        KeyedDecodingContainer(try TacKeyedValueDecoder<Key>(value, codingPath))
    }

    func unkeyedContainer() throws -> UnkeyedDecodingContainer {
        try TacListValueDecoder(value, codingPath)
    }

    func singleValueContainer() throws -> SingleValueDecodingContainer {
        TacSingleValueDecoder(value, codingPath)
    }
}
