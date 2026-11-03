import Foundation

final class NativeUiCssMask {
    private static let blank: Unicode.Scalar = " "
    private static let identifier: Unicode.Scalar = "_"
    private static let hexadecimalDigits = 6
    private static let whitespace: Set<Unicode.Scalar> = [" ", "\t", "\n", "\r", "\u{000c}"]
    private static let newlines: Set<Unicode.Scalar> = ["\n", "\r", "\u{000c}"]
    private static let nameLetters = Set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-@#".unicodeScalars)
    private static let urlRefused = Set("\"'(\\\u{007f}".unicodeScalars)
    private let scalars: [Unicode.Scalar]
    private var output: [Unicode.Scalar] = []
    private var mode = NativeUiCssMode.code
    private var quote: Unicode.Scalar = "'"
    private var commentPrevious: Unicode.Scalar?
    private var escapeRemaining = 0

    private init(_ source: String) { scalars = Array(source.unicodeScalars) }

    static func read(_ source: String) throws -> String {
        let reader = NativeUiCssMask(source)
        let masked = try reader.masked()
        try NativeUiRuntimeFailure.require(reader.mode == .code, "Unclosed CSS comment, string or url")
        return masked
    }

    private func masked() throws -> String {
        try scalars.indices.forEach { output.append(try step($0)) }
        return String(String.UnicodeScalarView(output))
    }

    private func step(_ at: Int) throws -> Unicode.Scalar {
        let operations: [NativeUiCssMode: (Int) throws -> Unicode.Scalar] = [
            .code: code, .opening: opened, .comment: comment, .quoted: quoted, .escaped: escaped,
            .identifier: escapedName, .urlStart: urlStart, .url: url, .urlEnd: urlEnd,
        ]
        guard let operation = operations[mode] else { throw NativeUiRuntimeFailure(description: "Invalid CSS reader state") }
        return try operation(at)
    }

    private func code(_ at: Int) throws -> Unicode.Scalar {
        let operations: [Unicode.Scalar: (Int) throws -> Unicode.Scalar] = [
            "/": slash, "'": beginQuote, "\"": beginQuote, "\\": beginEscape, "(": parenthesis,
        ]
        return try operations[scalars[at]].map { try $0(at) } ?? scalars[at]
    }

    private func slash(_ at: Int) -> Unicode.Scalar {
        switch scalars[(at + 1)...].first {
        case "*": mode = .opening; return Self.blank
        default: return "/"
        }
    }

    private func opened(_ at: Int) -> Unicode.Scalar {
        mode = .comment
        commentPrevious = nil
        return Self.blank
    }

    private func comment(_ at: Int) -> Unicode.Scalar {
        mode = commentPrevious == "*" && scalars[at] == "/" ? .code : .comment
        commentPrevious = scalars[at]
        return Self.blank
    }

    private func beginQuote(_ at: Int) -> Unicode.Scalar {
        quote = scalars[at]
        mode = .quoted
        return quote
    }

    private func quoted(_ at: Int) throws -> Unicode.Scalar {
        switch scalars[at] {
        case "\\": mode = .escaped
        case quote: mode = .code; return quote
        case "\n", "\r", "\u{000c}": throw NativeUiRuntimeFailure(description: "Unescaped newline in CSS string")
        default: break
        }
        return Self.blank
    }

    private func escaped(_ at: Int) -> Unicode.Scalar { mode = .quoted; return Self.blank }

    private func beginEscape(_ at: Int) throws -> Unicode.Scalar {
        escapeRemaining = try escapeLength(at) - 1
        mode = .identifier
        return Self.identifier
    }

    private func escapedName(_ at: Int) -> Unicode.Scalar {
        escapeRemaining -= 1
        mode = escapeRemaining == 0 ? .code : .identifier
        return Self.identifier
    }

    private func escapeLength(_ at: Int) throws -> Int {
        let next = scalar(at + 1)
        try NativeUiRuntimeFailure.require(next.map { !Self.newlines.contains($0) } ?? false, "CSS escape needs a following character")
        return 1 + escapedLength(at + 1)
    }

    private func escapedLength(_ start: Int) -> Int {
        let digits = scalars[start...].prefix(Self.hexadecimalDigits).prefix { $0.properties.isASCIIHexDigit }.count
        return digits == 0 ? 1 : digits + spacing(start + digits)
    }

    private func spacing(_ at: Int) -> Int {
        switch (scalar(at), scalar(at + 1)) {
        case ("\r", "\n"): return 2
        case (.some(let value), _) where Self.whitespace.contains(value): return 1
        default: return 0
        }
    }

    private func parenthesis(_ at: Int) -> Unicode.Scalar {
        if urlOpening() { mode = .urlStart }
        return "("
    }

    private func urlOpening() -> Bool {
        let name = String(String.UnicodeScalarView(output.suffix(3))).lowercased()
        let before = output.dropLast(3).last
        return name == "url" && !(before.map { Self.nameLetters.contains($0) } ?? false)
    }

    private func urlStart(_ at: Int) throws -> Unicode.Scalar {
        switch scalars[at] {
        case "'", "\"": return beginQuote(at)
        case ")": mode = .code; return ")"
        case let value where Self.whitespace.contains(value): return Self.blank
        default: return try urlBody(at)
        }
    }

    private func url(_ at: Int) throws -> Unicode.Scalar {
        switch scalars[at] {
        case ")": mode = .code; return ")"
        case let value where Self.whitespace.contains(value): mode = .urlEnd; return Self.blank
        default: return try urlBody(at)
        }
    }

    private func urlBody(_ at: Int) throws -> Unicode.Scalar {
        let value = scalars[at]
        try NativeUiRuntimeFailure.require(value.value > 32 && !Self.urlRefused.contains(value), "Malformed CSS url")
        mode = .url
        return Self.blank
    }

    private func urlEnd(_ at: Int) throws -> Unicode.Scalar {
        switch scalars[at] {
        case ")": mode = .code; return ")"
        case let value where Self.whitespace.contains(value): return Self.blank
        default: throw NativeUiRuntimeFailure(description: "Malformed CSS url")
        }
    }

    private func scalar(_ at: Int) -> Unicode.Scalar? { scalars.indices.contains(at) ? scalars[at] : nil }
}
