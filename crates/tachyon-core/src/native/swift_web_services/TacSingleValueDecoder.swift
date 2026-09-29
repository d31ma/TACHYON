struct TacSingleValueDecoder: SingleValueDecodingContainer {
    let value: Any?
    let codingPath: [CodingKey]

    init(_ value: Any?, _ codingPath: [CodingKey]) {
        self.value = value
        self.codingPath = codingPath
    }

    func decodeNil() -> Bool {
        value == nil
    }

    func decode<T: Decodable>(_ type: T.Type) throws -> T {
        if let same = value as? T { return same }
        if let number = number(type) { return number }
        guard !Self.isPlain(type) else { throw mismatch(type) }
        return try T(from: TacValueDecoder(value, codingPath))
    }

    private func number<T>(_ type: T.Type) -> T? {
        guard let real = Self.real(value) else { return nil }
        if let integer = type as? any BinaryInteger.Type {
            return Int64(exactly: real).flatMap { integer.init(exactly: $0) } as? T
        }
        guard let floating = type as? any BinaryFloatingPoint.Type else { return nil }
        return floating.init(real) as? T
    }

    private static func real(_ value: Any?) -> Double? {
        if let whole = value as? Int { return Double(whole) }
        return value as? Double
    }

    private static func isPlain<T>(_ type: T.Type) -> Bool {
        type is any BinaryInteger.Type || type is any BinaryFloatingPoint.Type
            || type == String.self || type == Bool.self
    }

    private func mismatch<T>(_ type: T.Type) -> DecodingError {
        DecodingError.typeMismatch(type, DecodingError.Context(
            codingPath: codingPath, debugDescription: "The value does not fit \(type)."))
    }
}
