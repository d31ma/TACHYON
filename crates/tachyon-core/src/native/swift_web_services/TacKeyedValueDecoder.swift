struct TacKeyedValueDecoder<Key: CodingKey>: KeyedDecodingContainerProtocol {
    let fields: [String: Any?]
    let codingPath: [CodingKey]

    init(_ value: Any?, _ codingPath: [CodingKey]) throws {
        guard let fields = value as? [String: Any?] else {
            throw DecodingError.typeMismatch([String: Any?].self, DecodingError.Context(
                codingPath: codingPath, debugDescription: "The value is not an object."))
        }
        self.fields = fields
        self.codingPath = codingPath
    }

    var allKeys: [Key] {
        fields.keys.compactMap { Key(stringValue: $0) }
    }

    func contains(_ key: Key) -> Bool {
        fields[key.stringValue] != nil
    }

    func decodeNil(forKey key: Key) throws -> Bool {
        try field(key) == nil
    }

    func decode<T: Decodable>(_ type: T.Type, forKey key: Key) throws -> T {
        try TacSingleValueDecoder(try field(key), codingPath + [key]).decode(type)
    }

    func nestedContainer<NestedKey: CodingKey>(
        keyedBy type: NestedKey.Type, forKey key: Key
    ) throws -> KeyedDecodingContainer<NestedKey> {
        try decoder(key).container(keyedBy: type)
    }

    func nestedUnkeyedContainer(forKey key: Key) throws -> UnkeyedDecodingContainer {
        try decoder(key).unkeyedContainer()
    }

    func superDecoder() throws -> Decoder {
        TacValueDecoder(fields, codingPath)
    }

    func superDecoder(forKey key: Key) throws -> Decoder {
        try decoder(key)
    }

    private func decoder(_ key: Key) throws -> TacValueDecoder {
        TacValueDecoder(try field(key), codingPath + [key])
    }

    private func field(_ key: Key) throws -> Any? {
        guard let found = fields[key.stringValue] else {
            throw DecodingError.keyNotFound(key, DecodingError.Context(
                codingPath: codingPath, debugDescription: "The object has no field " + key.stringValue + "."))
        }
        return found
    }
}
