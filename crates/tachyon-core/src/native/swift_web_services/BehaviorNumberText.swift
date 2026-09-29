struct BehaviorNumberText {
    private let bytes: [UInt8]
    private var index = 0

    init(_ text: String) {
        bytes = Array(text.utf8)
    }

    var isInteger: Bool {
        var reader = self
        return reader.readWhole() && reader.isAtEnd
    }

    var isNumber: Bool {
        var reader = self
        return reader.readWhole() && reader.readFraction() && reader.readExponent() && reader.isAtEnd
    }

    private var isAtEnd: Bool { index == bytes.count }

    private mutating func readWhole() -> Bool {
        _ = take("-")
        if take("0") { return true }
        guard let first = next, (UInt8(ascii: "1")...UInt8(ascii: "9")).contains(first) else { return false }
        _ = readDigits()
        return true
    }

    private mutating func readFraction() -> Bool {
        guard take(".") else { return true }
        return readDigits() > 0
    }

    private mutating func readExponent() -> Bool {
        guard take("e") || take("E") else { return true }
        _ = take("+") || take("-")
        return readDigits() > 0
    }

    private mutating func readDigits() -> Int {
        let start = index
        while let byte = next, (UInt8(ascii: "0")...UInt8(ascii: "9")).contains(byte) { index += 1 }
        return index - start
    }

    private var next: UInt8? { index < bytes.count ? bytes[index] : nil }

    private mutating func take(_ character: Unicode.Scalar) -> Bool {
        guard next == UInt8(ascii: character) else { return false }
        index += 1
        return true
    }
}
