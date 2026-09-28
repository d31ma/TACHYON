struct TacJSONReader {
    struct Null {}

    private static let deepest = 64
    private let bytes: [UInt8]
    private var index = 0
    private var depth = 0

    init(_ text: String) {
        bytes = Array(text.utf8)
    }

    mutating func document() -> Any? {
        guard let found = value() else { return nil }
        skipSpace()
        return index == bytes.count ? found : nil
    }

    private mutating func value() -> Any? {
        skipSpace()
        guard index < bytes.count else { return nil }
        switch bytes[index] {
        case UInt8(ascii: "{"): return nested { $0.object() }
        case UInt8(ascii: "["): return nested { $0.array() }
        case UInt8(ascii: "\""): return string()
        case UInt8(ascii: "t"): return word("true", true)
        case UInt8(ascii: "f"): return word("false", false)
        case UInt8(ascii: "n"): return word("null", Null())
        default: return number()
        }
    }

    private mutating func nested(_ read: (inout TacJSONReader) -> Any?) -> Any? {
        guard depth < Self.deepest else { return nil }
        depth += 1
        defer { depth -= 1 }
        index += 1
        return read(&self)
    }

    private mutating func object() -> Any? {
        var members: [String: Any] = [:]
        if take(UInt8(ascii: "}")) { return members }
        repeat {
            skipSpace()
            guard let name = string(), take(UInt8(ascii: ":")), members[name] == nil,
                  let member = value() else { return nil }
            members[name] = member
        } while take(UInt8(ascii: ","))
        return take(UInt8(ascii: "}")) ? members : nil
    }

    private mutating func array() -> Any? {
        var items: [Any] = []
        if take(UInt8(ascii: "]")) { return items }
        repeat {
            guard let item = value() else { return nil }
            items.append(item)
        } while take(UInt8(ascii: ","))
        return take(UInt8(ascii: "]")) ? items : nil
    }

    private mutating func string() -> String? {
        guard take(UInt8(ascii: "\"")) else { return nil }
        var text: [UInt8] = []
        while index < bytes.count {
            let byte = bytes[index]
            index += 1
            if byte == UInt8(ascii: "\"") { return String(decoding: text, as: UTF8.self) }
            if byte < 0x20 { return nil }
            if byte != UInt8(ascii: "\\") {
                text.append(byte)
                continue
            }
            guard let escaped = escape() else { return nil }
            text.append(contentsOf: escaped)
        }
        return nil
    }

    private mutating func escape() -> [UInt8]? {
        guard index < bytes.count else { return nil }
        let byte = bytes[index]
        index += 1
        switch byte {
        case UInt8(ascii: "\""), UInt8(ascii: "\\"), UInt8(ascii: "/"): return [byte]
        case UInt8(ascii: "b"): return [0x08]
        case UInt8(ascii: "f"): return [0x0C]
        case UInt8(ascii: "n"): return [0x0A]
        case UInt8(ascii: "r"): return [0x0D]
        case UInt8(ascii: "t"): return [0x09]
        case UInt8(ascii: "u"): return unicode()
        default: return nil
        }
    }

    private mutating func unicode() -> [UInt8]? {
        guard let first = hexadecimal() else { return nil }
        var code = first
        if (0xD800..<0xDC00).contains(first) {
            guard take(UInt8(ascii: "\\")), take(UInt8(ascii: "u")), let second = hexadecimal(),
                  (0xDC00..<0xE000).contains(second) else { return nil }
            code = 0x10000 + ((first - 0xD800) << 10) + (second - 0xDC00)
        }
        guard let scalar = Unicode.Scalar(code) else { return nil }
        return Array(String(Character(scalar)).utf8)
    }

    private mutating func hexadecimal() -> UInt32? {
        guard index + 4 <= bytes.count else { return nil }
        let digits = String(decoding: bytes[index..<index + 4], as: UTF8.self)
        index += 4
        return UInt32(digits, radix: 16)
    }

    private mutating func number() -> Any? {
        let start = index
        while index < bytes.count, "+-0123456789.eE".utf8.contains(bytes[index]) { index += 1 }
        let text = String(decoding: bytes[start..<index], as: UTF8.self)
        if let whole = Int(text) { return whole }
        guard let real = Double(text), real.isFinite else { return nil }
        return real
    }

    private mutating func word(_ spelled: String, _ meaning: Any) -> Any? {
        let wanted = Array(spelled.utf8)
        guard bytes.count - index >= wanted.count,
              Array(bytes[index..<index + wanted.count]) == wanted else { return nil }
        index += wanted.count
        return meaning
    }

    private mutating func take(_ byte: UInt8) -> Bool {
        skipSpace()
        guard index < bytes.count, bytes[index] == byte else { return false }
        index += 1
        return true
    }

    private mutating func skipSpace() {
        while index < bytes.count, [0x09, 0x0A, 0x0D, 0x20].contains(bytes[index]) { index += 1 }
    }
}
