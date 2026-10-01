import Foundation

struct NativeUiCssGrammar {
    private static let rules: Set<String> = ["media", "layer", "supports", "container", "keyframes", "view-transition"]
    private static let functions: Set<String> = [
        "calc", "clamp", "min", "max", "minmax", "repeat",
        "rgb", "rgba", "hsl", "hsla", "color-mix", "linear-gradient", "radial-gradient",
        "repeating-linear-gradient", "repeating-radial-gradient", "repeating-conic-gradient",
        "cubic-bezier", "steps", "blur", "brightness", "saturate", "invert", "sepia",
        "rotate", "scale", "scalex", "scaley", "translate", "translatex", "translatey", "skewy",
        "is", "where", "not", "has", "nth-child", "nth-of-type", "lang", "and", "or",
    ]

    static func validate(_ source: String) throws {
        try ascii(source)
        try balanced(source)
        try vocabulary(source)
    }

    private static func ascii(_ source: String) throws {
        let safe = source.unicodeScalars.allSatisfy { $0.value < 128 && $0.value != 0 }
        try NativeUiRuntimeFailure.require(safe, "Non-ASCII active CSS code needs a separate lexical profile")
    }

    private static func balanced(_ source: String) throws {
        var stack: [Unicode.Scalar] = []
        try source.unicodeScalars.forEach { try delimiter($0, &stack) }
        try NativeUiRuntimeFailure.require(stack.isEmpty, "Unclosed CSS delimiter")
    }

    private static func delimiter(_ scalar: Unicode.Scalar, _ stack: inout [Unicode.Scalar]) throws {
        let opening: [Unicode.Scalar: Unicode.Scalar] = ["(": ")", "[": "]", "{": "}"]
        try opening[scalar].map { try push($0, &stack) } ?? closing(scalar, &stack)
    }

    private static func push(_ close: Unicode.Scalar, _ stack: inout [Unicode.Scalar]) throws {
        try NativeUiRuntimeFailure.require(stack.count < 64, "CSS delimiter depth exceeded")
        stack.append(close)
    }

    private static func closing(_ scalar: Unicode.Scalar, _ stack: inout [Unicode.Scalar]) throws {
        if [")", "]", "}"].contains(scalar) {
            try NativeUiRuntimeFailure.require(stack.popLast() == scalar, "Mismatched CSS delimiter")
        }
    }

    private static func vocabulary(_ source: String) throws {
        let pattern = try NSRegularExpression(pattern: "@([A-Za-z_-][A-Za-z0-9_-]*)?|([A-Za-z_-][A-Za-z0-9_-]*)\\(")
        let range = NSRange(source.startIndex..., in: source)
        let tokens = pattern.matches(in: source, range: range)
        try NativeUiRuntimeFailure.require(tokens.count <= 16384, "CSS grammar token bound exceeded")
        try tokens.forEach { try token($0, source) }
    }

    private static func word(_ text: String) -> (String, Set<String>) {
        switch text.first {
        case "@": return (String(text.dropFirst()), rules)
        default: return (String(text.prefix { $0 != "(" }).trimmingCharacters(in: .whitespacesAndNewlines), functions)
        }
    }

    private static func token(_ match: NSTextCheckingResult, _ source: String) throws {
        let text = (source as NSString).substring(with: match.range)
        let (name, vocabulary) = word(text)
        try NativeUiRuntimeFailure.require(vocabulary.contains(name.lowercased()), "Unsupported CSS rule or function: \(name)")
    }
}
