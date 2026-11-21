
struct AppleCryptoValues {
    static let mostBytes = 32768
    static let mostPayloadBytes = 65536
    static let mostNameBytes = 64
    static let leastSaltBytes = 16
    static let mostCodeBytes = 1024
    static let mostDerivedBytes = 1024
    static let largestKeyId = 9_007_199_254_740_991.0
    static let base64Letters = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_")
    static let nameLetters = base64Letters.union(CharacterSet(charactersIn: "."))
    let values: [Any]

    static func parsed(_ payload: String, _ count: Int) throws -> AppleCryptoValues {
        guard payload.utf8.count <= mostPayloadBytes,
              let values = (try? JSONSerialization.jsonObject(with: Data(payload.utf8))) as? [Any],
              values.count == count else { throw AppleCryptoRefusal.invalid }
        return AppleCryptoValues(values: values)
    }

    static func encoded(_ data: Data) -> String {
        data.base64EncodedString()
            .replacingOccurrences(of: "+", with: "-")
            .replacingOccurrences(of: "/", with: "_")
            .replacingOccurrences(of: "=", with: "")
    }

    func name(_ index: Int) throws -> String {
        let text = try self.text(index)
        guard (1...Self.mostNameBytes).contains(text.utf8.count), Self.only(text, Self.nameLetters) else {
            throw AppleCryptoRefusal.invalid
        }
        return text
    }

    func bytes(_ index: Int, most: Int = mostBytes) throws -> Data {
        let text = try self.text(index)
        guard Self.only(text, Self.base64Letters), text.utf8.count % 4 != 1,
              let data = Data(base64Encoded: Self.standard(text)), data.count <= most else {
            throw AppleCryptoRefusal.invalid
        }
        return data
    }

    func sized(_ index: Int, _ size: Int) throws -> Data {
        let data = try bytes(index)
        guard data.count == size else { throw AppleCryptoRefusal.invalid }
        return data
    }

    func salt(_ index: Int) throws -> Data {
        let data = try bytes(index)
        guard data.count >= Self.leastSaltBytes else { throw AppleCryptoRefusal.invalid }
        return data
    }

    func code(_ index: Int) throws -> Data {
        let data = Data(try text(index).utf8)
        guard (1...Self.mostCodeBytes).contains(data.count) else { throw AppleCryptoRefusal.invalid }
        return data
    }

    func choice(_ index: Int, _ allowed: [String]) throws -> String {
        let text = try self.text(index)
        guard allowed.contains(text) else { throw AppleCryptoRefusal.invalid }
        return text
    }

    func flag(_ index: Int) throws -> Bool {
        guard let number = values[index] as? NSNumber, CFGetTypeID(number) == CFBooleanGetTypeID() else {
            throw AppleCryptoRefusal.invalid
        }
        return number.boolValue
    }

    func length(_ index: Int) throws -> Int {
        guard let number = values[index] as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
              number.doubleValue == Double(number.intValue),
              (1...Self.mostDerivedBytes).contains(number.intValue) else { throw AppleCryptoRefusal.invalid }
        return number.intValue
    }

    func keyId(_ index: Int) throws -> UInt64 {
        guard let number = values[index] as? NSNumber, CFGetTypeID(number) != CFBooleanGetTypeID(),
              number.doubleValue == number.doubleValue.rounded(),
              (0...Self.largestKeyId).contains(number.doubleValue) else { throw AppleCryptoRefusal.invalid }
        return number.uint64Value
    }

    private func text(_ index: Int) throws -> String {
        guard let text = values[index] as? String else { throw AppleCryptoRefusal.invalid }
        return text
    }

    private static func only(_ text: String, _ letters: CharacterSet) -> Bool {
        text.unicodeScalars.allSatisfy { letters.contains($0) }
    }

    private static func standard(_ text: String) -> String {
        let plain = text.replacingOccurrences(of: "-", with: "+").replacingOccurrences(of: "_", with: "/")
        return plain + String(repeating: "=", count: (4 - plain.utf8.count % 4) % 4)
    }
}
