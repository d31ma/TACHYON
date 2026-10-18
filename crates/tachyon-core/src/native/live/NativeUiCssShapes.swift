import Foundation

struct NativeUiCssShapes {
    private static let forms: [(name: String, found: String, valid: String)] = [
        ("var()", "(?<![A-Za-z0-9_-])var\\(", "(?<![A-Za-z0-9_-])var\\(\\s*--[A-Za-z0-9_-]+\\s*[,)]"),
        ("@property", "@property(?![A-Za-z0-9_-])", "@property\\s+--[A-Za-z0-9_-]+\\s*\\{"),
        ("@font-face", "@font-face(?![A-Za-z0-9_-])", "@font-face\\s*\\{"),
    ]

    static func validate(_ text: NativeUiCssText) throws {
        try forms.forEach { try form($0, text) }
    }

    private static func form(_ form: (name: String, found: String, valid: String), _ text: NativeUiCssText) throws {
        let found = try text.matches(form.found).count
        let valid = try text.matches(form.valid).count
        try NativeUiRuntimeFailure.require(found == valid, "Unsupported CSS \(form.name) form")
    }
}
