import Foundation

struct NativeUiCssReference {
    private static let letters = Set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789._~@+-/".unicodeScalars)
    private static let fragmentLetters = Set("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-".unicodeScalars)

    static func resource(_ text: String, _ sheet: NativeUiCssSheet) throws { try checked(text, sheet) }

    static func stylesheet(_ text: String, _ sheet: NativeUiCssSheet) throws -> String {
        let target = try path(text, sheet)
        try NativeUiRuntimeFailure.require(sheet.stylesheet(target), "CSS @import must name a captured stylesheet")
        return target
    }

    private static func checked(_ text: String, _ sheet: NativeUiCssSheet) throws {
        switch text.hasPrefix("#") {
        case true: try fragmentName(text)
        case false: _ = try path(text, sheet)
        }
    }

    private static func fragmentName(_ text: String) throws {
        let name = text.unicodeScalars.dropFirst()
        try NativeUiRuntimeFailure.require(!name.isEmpty && name.allSatisfy(fragmentLetters.contains), "CSS url fragment must be a plain document identifier")
    }

    private static func path(_ text: String, _ sheet: NativeUiCssSheet) throws -> String {
        try spelling(text)
        let target = try resolved(text, sheet)
        try NativeUiRuntimeFailure.require(sheet.captured(target), "CSS url must name an asset in the captured generation")
        return target
    }

    private static func spelling(_ text: String) throws {
        try NativeUiRuntimeFailure.require((1...NativeUiRuntimeBounds.pathBytes).contains(text.utf8.count), "CSS url length is outside the captured path bound")
        try NativeUiRuntimeFailure.require(text.unicodeScalars.allSatisfy(letters.contains), "CSS url must be a plain relative path")
        try NativeUiRuntimeFailure.require(!text.hasPrefix("/"), "CSS url must be relative to its stylesheet")
    }

    private static func resolved(_ text: String, _ sheet: NativeUiCssSheet) throws -> String {
        var parts = sheet.folder()
        try text.split(separator: "/", omittingEmptySubsequences: false).forEach { try walk($0, &parts) }
        return parts.joined(separator: "/")
    }

    private static func walk(_ part: Substring, _ parts: inout [Substring]) throws {
        switch part {
        case "": throw NativeUiRuntimeFailure(description: "CSS url has an empty path segment")
        case ".": return
        case "..": try NativeUiRuntimeFailure.require(parts.popLast() != nil, "CSS url leaves the captured generation")
        default: parts.append(part)
        }
    }
}
