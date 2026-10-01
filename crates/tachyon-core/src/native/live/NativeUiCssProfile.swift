import Foundation

struct NativeUiCssProfile {
    static func validate(_ bytes: Data) throws {
        let source = try utf8(bytes)
        let masked = try NativeUiCssMask.read(source)
        try references(masked)
    }

    private static func utf8(_ bytes: Data) throws -> String {
        guard let source = String(data: bytes, encoding: .utf8) else { throw NativeUiRuntimeFailure(description: "Stylesheet must be UTF-8") }
        return source
    }

    private static func references(_ source: String) throws {
        let pattern = try NSRegularExpression(pattern: "[A-Za-z_-][A-Za-z0-9_-]*\\s*\\(", options: .caseInsensitive)
        let range = NSRange(source.startIndex..., in: source)
        try NativeUiRuntimeFailure.require(pattern.firstMatch(in: source, range: range) == nil, "CSS functions need a separately qualified dependency profile")
    }
}
