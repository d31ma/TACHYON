struct TacListValueDecoder: UnkeyedDecodingContainer {
    let items: [Any?]
    let codingPath: [CodingKey]
    private(set) var currentIndex = 0

    init(_ value: Any?, _ codingPath: [CodingKey]) throws {
        guard let items = value as? [Any?] else {
            throw DecodingError.typeMismatch([Any?].self, DecodingError.Context(
                codingPath: codingPath, debugDescription: "The value is not a list."))
        }
        self.items = items
        self.codingPath = codingPath
    }

    var count: Int? { items.count }

    var isAtEnd: Bool { currentIndex >= items.count }

    mutating func decodeNil() throws -> Bool {
        guard try peek() == nil else { return false }
        currentIndex += 1
        return true
    }

    mutating func decode<T: Decodable>(_ type: T.Type) throws -> T {
        try TacSingleValueDecoder(try next(), codingPath).decode(type)
    }

    mutating func nestedContainer<NestedKey: CodingKey>(
        keyedBy type: NestedKey.Type
    ) throws -> KeyedDecodingContainer<NestedKey> {
        try TacValueDecoder(try next(), codingPath).container(keyedBy: type)
    }

    mutating func nestedUnkeyedContainer() throws -> UnkeyedDecodingContainer {
        try TacValueDecoder(try next(), codingPath).unkeyedContainer()
    }

    mutating func superDecoder() throws -> Decoder {
        TacValueDecoder(try next(), codingPath)
    }

    private mutating func next() throws -> Any? {
        let found = try peek()
        currentIndex += 1
        return found
    }

    private func peek() throws -> Any? {
        guard !isAtEnd else {
            throw DecodingError.valueNotFound(Any.self, DecodingError.Context(
                codingPath: codingPath, debugDescription: "The list has no more items."))
        }
        return items[currentIndex]
    }
}
