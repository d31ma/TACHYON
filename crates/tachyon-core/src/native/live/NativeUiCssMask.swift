import Foundation

final class NativeUiCssMask {
    private let scalars: [Unicode.Scalar]
    private var mode = NativeUiCssMode.code
    private var quote: Unicode.Scalar = "'"
    private var commentPrevious: Unicode.Scalar?

    private init(_ source: String) { scalars = Array(source.unicodeScalars) }

    static func read(_ source: String) throws -> String {
        let reader = NativeUiCssMask(source)
        let masked = try reader.masked()
        try NativeUiRuntimeFailure.require(reader.mode == .code, "Unclosed CSS comment or string")
        return masked
    }

    private func masked() throws -> String {
        let values = try scalars.indices.map { try step($0) }
        return String(String.UnicodeScalarView(values))
    }

    private func step(_ at: Int) throws -> Unicode.Scalar {
        let operations: [NativeUiCssMode: (Int) throws -> Unicode.Scalar] = [
            .code: code, .opening: opened, .comment: comment, .quoted: quoted, .escaped: escaped,
        ]
        guard let operation = operations[mode] else { throw NativeUiRuntimeFailure(description: "Invalid CSS reader state") }
        return try operation(at)
    }

    private func code(_ at: Int) throws -> Unicode.Scalar {
        let operations: [Unicode.Scalar: (Int) throws -> Unicode.Scalar] = [
            "/": slash, "'": beginQuote, "\"": beginQuote, "\\": refused,
        ]
        return try operations[scalars[at]].map { try $0(at) } ?? scalars[at]
    }

    private func slash(_ at: Int) -> Unicode.Scalar {
        switch scalars[(at + 1)...].first {
        case "*": mode = .opening; return " "
        default: return "/"
        }
    }

    private func opened(_ at: Int) -> Unicode.Scalar {
        mode = .comment
        commentPrevious = nil
        return " "
    }

    private func comment(_ at: Int) -> Unicode.Scalar {
        mode = commentPrevious == "*" && scalars[at] == "/" ? .code : .comment
        commentPrevious = scalars[at]
        return " "
    }

    private func beginQuote(_ at: Int) -> Unicode.Scalar {
        quote = scalars[at]
        mode = .quoted
        return " "
    }

    private func quoted(_ at: Int) throws -> Unicode.Scalar {
        switch scalars[at] {
        case "\\": mode = .escaped
        case quote: mode = .code
        case "\n", "\r", "\u{000c}": throw NativeUiRuntimeFailure(description: "Unescaped newline in CSS string")
        default: break
        }
        return " "
    }

    private func escaped(_ at: Int) -> Unicode.Scalar { mode = .quoted; return " " }

    private func refused(_ at: Int) throws -> Unicode.Scalar {
        throw NativeUiRuntimeFailure(description: "Escaped CSS code is not supported by this style profile")
    }
}
