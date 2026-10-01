import Foundation

struct NativeUiStyleAddress {
    static let prefix = "/.tachyon-native-ui/"

    static func url(_ generation: String, _ path: String) throws -> URL {
        try NativeUiRuntimeJson.digest(generation)
        try NativeUiRuntimeAsset.canonicalPath(path)
        var parts = URLComponents()
        parts.scheme = "tachyon-app"
        parts.host = "bundle"
        parts.path = prefix + generation + "/" + path
        guard let url = parts.url else { throw NativeUiRuntimeFailure(description: "Invalid generation asset URL") }
        return url
    }

    static func path(_ url: URL) throws -> String {
        try origin(url)
        try encodedPath(url)
        let path = String(url.path.dropFirst())
        try NativeUiRuntimeAsset.canonicalPath(path)
        return path
    }

    private static func origin(_ url: URL) throws {
        let authority = url.scheme == "tachyon-app" && url.host == "bundle"
            && url.port == nil && url.user == nil && url.password == nil
            && url.query == nil && url.fragment == nil
        try NativeUiRuntimeFailure.require(authority, "Only local captured generation assets are allowed")
    }

    private static func encodedPath(_ url: URL) throws {
        let encoded = URLComponents(url: url, resolvingAgainstBaseURL: false)?.percentEncodedPath.lowercased() ?? ""
        let ambiguous = ["%2f", "%5c", "%00"].contains { encoded.contains($0) }
        try NativeUiRuntimeFailure.require(!ambiguous, "Ambiguous asset URL encoding")
    }
}
