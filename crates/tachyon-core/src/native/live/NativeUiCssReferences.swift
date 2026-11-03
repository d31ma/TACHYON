import Foundation

final class NativeUiCssReferences {
    private static let urlPattern = "(?<![A-Za-z0-9_-])url\\("
    private static let importPattern = "@import(?![A-Za-z0-9_-])"
    private static let urlName = "url("
    private static let urlLimit = 1024
    private static let importLimit = 64
    private static let quotes: Set<Unicode.Scalar> = ["'", "\""]
    private static let closing: Set<Unicode.Scalar> = [")"]
    private static let endings: Set<Unicode.Scalar> = [";", "{", "}"]
    private static let whitespace: Set<Unicode.Scalar> = [" ", "\t", "\n", "\r", "\u{000c}"]
    private let text: NativeUiCssText
    private let sheet: NativeUiCssSheet

    private init(_ text: NativeUiCssText, _ sheet: NativeUiCssSheet) {
        self.text = text
        self.sheet = sheet
    }

    static func read(_ text: NativeUiCssText, _ sheet: NativeUiCssSheet) throws -> [String] {
        let reader = NativeUiCssReferences(text, sheet)
        try reader.resources()
        return try reader.imports()
    }

    private func resources() throws {
        let openings = try text.matches(Self.urlPattern)
        try NativeUiRuntimeFailure.require(openings.count <= Self.urlLimit, "CSS url bound exceeded")
        try openings.forEach { try NativeUiCssReference.resource(try argument($0 - 1).value, sheet) }
    }

    private func imports() throws -> [String] {
        let keywords = try text.matches(Self.importPattern)
        try NativeUiRuntimeFailure.require(keywords.count <= Self.importLimit, "CSS @import bound exceeded")
        return try keywords.map { try importTarget($0) }
    }

    private func importTarget(_ end: Int) throws -> String {
        try NativeUiRuntimeFailure.require(text.depth(end) == 0, "CSS @import must be a top-level rule")
        let start = try required(text.written(end), "CSS @import needs a stylesheet")
        let target = try importValue(start)
        try terminated(target.end)
        return try NativeUiCssReference.stylesheet(target.value, sheet)
    }

    private func importValue(_ start: Int) throws -> (value: String, end: Int) {
        Self.quotes.contains(text.masked[start]) ? try importString(start) : try importUrl(start)
    }

    private func importString(_ start: Int) throws -> (value: String, end: Int) {
        let close = try required(text.next([text.masked[start]], start + 1), "Unclosed CSS @import string")
        return (text.originalText((start + 1)..<close), close + 1)
    }

    private func importUrl(_ start: Int) throws -> (value: String, end: Int) {
        let name = text.maskedText(start..<min(start + Self.urlName.count, text.masked.count))
        try NativeUiRuntimeFailure.require(name.lowercased() == Self.urlName, "CSS @import needs a string or url")
        return try argument(start + Self.urlName.count - 1)
    }

    private func terminated(_ start: Int) throws {
        let ending = try required(text.next(Self.endings, start), "Unterminated CSS @import")
        try NativeUiRuntimeFailure.require(text.masked[ending] == ";", "CSS @import must end with a semicolon")
    }

    private func argument(_ open: Int) throws -> (value: String, end: Int) {
        let close = try required(text.next(Self.closing, open + 1), "Unclosed CSS url")
        return (try value(try trimmed(open + 1, close)), close + 1)
    }

    private func value(_ range: Range<Int>) throws -> String {
        Self.quotes.contains(text.masked[range.lowerBound]) ? try quoted(range) : try unquoted(range)
    }

    private func trimmed(_ start: Int, _ end: Int) throws -> Range<Int> {
        let written = (start..<end).filter { !Self.whitespace.contains(text.original[$0]) }
        guard let first = written.first, let last = written.last else { throw NativeUiRuntimeFailure(description: "Empty CSS url") }
        return first..<(last + 1)
    }

    private func quoted(_ range: Range<Int>) throws -> String {
        let inner = (range.lowerBound + 1)..<max(range.lowerBound + 1, range.upperBound - 1)
        let closed = range.count > 1 && text.masked[range.upperBound - 1] == text.masked[range.lowerBound]
        try NativeUiRuntimeFailure.require(closed && blank(inner), "CSS url must hold exactly one string")
        return text.originalText(inner)
    }

    private func unquoted(_ range: Range<Int>) throws -> String {
        try NativeUiRuntimeFailure.require(blank(range), "Malformed CSS url")
        return text.originalText(range)
    }

    private func blank(_ range: Range<Int>) -> Bool {
        text.masked[range].allSatisfy { $0 == " " }
    }

    private func required<Value>(_ value: Value?, _ message: String) throws -> Value {
        guard let value else { throw NativeUiRuntimeFailure(description: message) }
        return value
    }
}
