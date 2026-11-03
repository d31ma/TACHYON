import Foundation

struct NativeUiCssProfile {
    static func validate(_ bytes: Data, _ sheet: NativeUiCssSheet) throws -> [String] {
        try bounded(bytes)
        let source = try utf8(bytes)
        let masked = try NativeUiCssMask.read(source)
        try NativeUiCssGrammar.validate(masked)
        let text = NativeUiCssText(source, masked)
        try NativeUiCssShapes.validate(text)
        return try NativeUiCssReferences.read(text, sheet)
    }

    private static func bounded(_ bytes: Data) throws {
        try NativeUiRuntimeFailure.require(bytes.count <= 1048576, "Stylesheet grammar profile byte bound exceeded")
    }

    private static func utf8(_ bytes: Data) throws -> String {
        guard let source = String(data: bytes, encoding: .utf8) else { throw NativeUiRuntimeFailure(description: "Stylesheet must be UTF-8") }
        return source
    }
}
