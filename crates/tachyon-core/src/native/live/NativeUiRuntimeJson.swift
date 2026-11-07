import Foundation
import CryptoKit

struct NativeUiRuntimeJson {
    static func quote(_ value: String) -> String {
        let body = value.unicodeScalars.map(escape).joined()
        return "\"" + body + "\""
    }

    static func sha256(_ bytes: Data) -> String {
        SHA256.hash(data: bytes).map { String(format: "%02x", $0) }.joined()
    }

    static func digest(_ value: String) throws {
        try NativeUiRuntimeFailure.require(value.utf8.count == 64, "Expected lowercase SHA-256")
        try NativeUiRuntimeFailure.require(value.utf8.allSatisfy(hexadecimal), "Expected lowercase SHA-256")
    }

    static func bytes(_ value: String) -> Data { Data(value.utf8) }

    private static func hexadecimal(_ byte: UInt8) -> Bool {
        (48...57).contains(byte) || (97...102).contains(byte)
    }

    private static func escape(_ scalar: Unicode.Scalar) -> String {
        switch scalar.value {
        case 8: return "\\b"
        case 9: return "\\t"
        case 10: return "\\n"
        case 12: return "\\f"
        case 13: return "\\r"
        case 34: return "\\\""
        case 92: return "\\\\"
        case 0...31: return String(format: "\\u%04x", scalar.value)
        default: return String(scalar)
        }
    }
}
