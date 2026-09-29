struct TacSingleValueEncoder: SingleValueEncodingContainer {
    private static let largestExactWhole = 9_007_199_254_740_992.0
    let node: TacEncodedNode
    let codingPath: [CodingKey]

    init(_ node: TacEncodedNode, _ codingPath: [CodingKey]) {
        self.node = node
        self.codingPath = codingPath
    }

    mutating func encodeNil() throws {
        node.value = nil
    }

    mutating func encode<T: Encodable>(_ value: T) throws {
        if let plain = try Self.plain(value, codingPath) {
            node.value = plain
            return
        }
        try value.encode(to: TacValueEncoder(node, codingPath))
    }

    private static func plain<T>(_ value: T, _ codingPath: [CodingKey]) throws -> Any? {
        if value is String || value is Bool { return value }
        if let integer = value as? any BinaryInteger { return try whole(integer, codingPath) }
        guard let floating = value as? any BinaryFloatingPoint else { return nil }
        return try finite(Double(floating), codingPath)
    }

    private static func whole(_ integer: any BinaryInteger, _ codingPath: [CodingKey]) throws -> Any {
        if let narrow = Int(exactly: integer) { return narrow }
        return try exactReal(integer, codingPath)
    }

    private static func exactReal(_ integer: any BinaryInteger, _ codingPath: [CodingKey]) throws -> Double {
        guard let real = Double(exactly: integer), real.magnitude <= largestExactWhole else {
            throw invalid(integer, codingPath)
        }
        return real
    }

    private static func finite(_ real: Double, _ codingPath: [CodingKey]) throws -> Double {
        guard real.isFinite else { throw invalid(real, codingPath) }
        return real
    }

    private static func invalid(_ value: Any, _ codingPath: [CodingKey]) -> EncodingError {
        EncodingError.invalidValue(value, EncodingError.Context(
            codingPath: codingPath, debugDescription: "JSON cannot hold this number."))
    }
}
