import Foundation

struct NativeUiRuntimeAsset: Decodable, Equatable {
    let path: String
    let size: UInt64
    let sha256: String

    func validate() throws {
        try Self.canonicalPath(path)
        try NativeUiRuntimeFailure.require(size <= UInt64(NativeUiRuntimeBounds.assetBytes), "Asset byte limit exceeded")
        try NativeUiRuntimeJson.digest(sha256)
    }

    func canonical() -> String {
        "{\"path\":" + NativeUiRuntimeJson.quote(path)
            + ",\"size\":" + String(size)
            + ",\"sha256\":" + NativeUiRuntimeJson.quote(sha256) + "}"
    }

    func stylesheet() -> Bool { (path as NSString).pathExtension == "css" }

    static func canonicalPath(_ path: String) throws {
        try pathLength(path)
        try pathLetters(path)
        try pathParts(path)
    }

    private static func pathLength(_ path: String) throws {
        try NativeUiRuntimeFailure.require((1...NativeUiRuntimeBounds.pathBytes).contains(path.utf8.count), "Invalid local asset path")
    }

    private static func pathLetters(_ path: String) throws {
        try NativeUiRuntimeFailure.require(!path.contains(where: { "\\:?#%".contains($0) }), "Ambiguous local asset path")
    }

    private static func pathParts(_ path: String) throws {
        try NativeUiRuntimeFailure.require(path.split(separator: "/", omittingEmptySubsequences: false).allSatisfy(pathPart), "Noncanonical local asset path")
    }

    private static func pathPart(_ part: Substring) -> Bool {
        !part.isEmpty && part != "." && part != ".."
    }
}
