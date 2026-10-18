import Foundation

struct NativeUiCssText {
    let original: [Unicode.Scalar]
    let masked: [Unicode.Scalar]
    let maskedSource: String

    init(_ source: String, _ masked: String) {
        original = Array(source.unicodeScalars)
        self.masked = Array(masked.unicodeScalars)
        maskedSource = masked
    }

    func matches(_ pattern: String) throws -> [Int] {
        let expression = try NSRegularExpression(pattern: pattern, options: [.caseInsensitive])
        let range = NSRange(maskedSource.startIndex..., in: maskedSource)
        return expression.matches(in: maskedSource, range: range).map { $0.range.location + $0.range.length }
    }

    func next(_ letters: Set<Unicode.Scalar>, _ start: Int) -> Int? {
        masked.indices.dropFirst(start).first { letters.contains(masked[$0]) }
    }

    func written(_ start: Int) -> Int? {
        masked.indices.dropFirst(start).first { masked[$0] != " " }
    }

    func originalText(_ range: Range<Int>) -> String {
        String(String.UnicodeScalarView(original[range]))
    }

    func maskedText(_ range: Range<Int>) -> String {
        String(String.UnicodeScalarView(masked[range]))
    }

    func depth(_ end: Int) -> Int {
        masked[..<end].reduce(0) { level, scalar in level + (scalar == "{" ? 1 : 0) - (scalar == "}" ? 1 : 0) }
    }
}
