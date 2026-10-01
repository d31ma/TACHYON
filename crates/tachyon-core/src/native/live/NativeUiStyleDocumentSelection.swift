import Foundation

@MainActor final class NativeUiStyleDocumentSelection {
    private static let urlBytes = 4096
    private static let queryBytes = 2048
    private let assets: NativeUiStyleAssets
    private let route: (String) -> String?

    init(_ assets: NativeUiStyleAssets, _ route: @escaping (String) -> String?) {
        self.assets = assets
        self.route = route
    }

    func document(_ url: URL) throws -> String {
        let parts = try components(url)
        try limits(url, parts)
        try pathname(parts)
        let normalized = try normalized(parts)
        try NativeUiStyleAddress.origin(normalized)
        let path = try mapped(parts.path)
        try captured(path)
        return path
    }

    private func components(_ url: URL) throws -> URLComponents {
        guard let parts = URLComponents(url: url, resolvingAgainstBaseURL: false) else {
            throw NativeUiRuntimeFailure(description: "Invalid owned document address")
        }
        return parts
    }

    private func limits(_ url: URL, _ parts: URLComponents) throws {
        try NativeUiRuntimeFailure.require(url.absoluteString.utf8.count <= Self.urlBytes, "Owned document URL limit exceeded")
        try NativeUiRuntimeFailure.require((parts.percentEncodedQuery ?? "").utf8.count <= Self.queryBytes, "Owned document query limit exceeded")
        try NativeUiRuntimeFailure.require(parts.fragment == nil, "Document fragments cannot select stylesheets")
    }

    private func normalized(_ source: URLComponents) throws -> URL {
        var parts = source
        parts.query = nil
        guard let url = parts.url else { throw NativeUiRuntimeFailure(description: "Invalid local document address") }
        return url
    }

    private func pathname(_ parts: URLComponents) throws {
        try encoded(parts.percentEncodedPath)
        try reserved(parts.path)
        try canonical(parts.path)
    }

    private func encoded(_ path: String) throws {
        let encoded = path.lowercased()
        let ambiguous = ["%2f", "%5c", "%00"].contains { encoded.contains($0) }
        try NativeUiRuntimeFailure.require(!ambiguous, "Ambiguous document URL encoding")
    }

    private func reserved(_ path: String) throws {
        try NativeUiRuntimeFailure.require(!path.hasPrefix(NativeUiStyleAddress.prefix), "Generation assets cannot select a document")
    }

    private func canonical(_ path: String) throws {
        switch path {
        case "", "/": return
        default: try directory(path)
        }
    }

    private func directory(_ path: String) throws {
        let name = path.trimmingCharacters(in: CharacterSet(charactersIn: "/"))
        try NativeUiRuntimeFailure.require(["/" + name, "/" + name + "/"].contains(path), "Noncanonical document route")
        try NativeUiRuntimeAsset.canonicalPath(name)
    }

    private func mapped(_ path: String) throws -> String {
        guard let document = route(path) else { throw NativeUiRuntimeFailure(description: "Owned document has no captured route") }
        return document
    }

    private func captured(_ path: String) throws {
        try NativeUiRuntimeAsset.canonicalPath(path)
        try NativeUiRuntimeFailure.require((path as NSString).pathExtension == "html", "Captured route is not a document")
        try NativeUiRuntimeFailure.require(assets.current.asset(path) != nil, "Owned document is not captured")
    }
}
