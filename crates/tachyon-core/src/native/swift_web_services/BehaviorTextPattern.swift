enum BehaviorTextPattern {
    static let shortName = "[A-Za-z_$][A-Za-z0-9_$]{0,63}"
    static let longName = "[A-Za-z_$][A-Za-z0-9_$]{0,127}"
    static let settingName = "[A-Z][A-Z0-9_]*"
    static let settingKey = "[a-z][a-z0-9]*"
    static let integer = "-?(0|[1-9][0-9]*)"
    static let number = "-?(0|[1-9][0-9]*)(\\.[0-9]+)?([eE][+-]?[0-9]+)?"
    private static let shortNameLength = 64
    private static let longNameLength = 128

    static func matches(_ value: String, _ pattern: String) -> Bool {
        switch pattern {
        case shortName: return word(value, shortNameLength, isNameStart, isNamePart)
        case longName: return word(value, longNameLength, isNameStart, isNamePart)
        case settingName: return word(value, Int.max, isUpper, isSettingPart)
        case settingKey: return word(value, Int.max, isLower, isKeyPart)
        case integer: return BehaviorNumberText(value).isInteger
        case number: return BehaviorNumberText(value).isNumber
        default: return false
        }
    }

    private static func word(
        _ value: String, _ longest: Int, _ first: (UInt8) -> Bool, _ rest: (UInt8) -> Bool
    ) -> Bool {
        let bytes = Array(value.utf8)
        guard let head = bytes.first, bytes.count <= longest, first(head) else { return false }
        return bytes.dropFirst().allSatisfy(rest)
    }

    private static func isNameStart(_ byte: UInt8) -> Bool {
        isUpper(byte) || isLower(byte) || byte == UInt8(ascii: "_") || byte == UInt8(ascii: "$")
    }

    private static func isNamePart(_ byte: UInt8) -> Bool {
        isNameStart(byte) || isDigit(byte)
    }

    private static func isSettingPart(_ byte: UInt8) -> Bool {
        isUpper(byte) || isDigit(byte) || byte == UInt8(ascii: "_")
    }

    private static func isKeyPart(_ byte: UInt8) -> Bool {
        isLower(byte) || isDigit(byte)
    }

    private static func isUpper(_ byte: UInt8) -> Bool {
        (UInt8(ascii: "A")...UInt8(ascii: "Z")).contains(byte)
    }

    private static func isLower(_ byte: UInt8) -> Bool {
        (UInt8(ascii: "a")...UInt8(ascii: "z")).contains(byte)
    }

    private static func isDigit(_ byte: UInt8) -> Bool {
        (UInt8(ascii: "0")...UInt8(ascii: "9")).contains(byte)
    }
}
