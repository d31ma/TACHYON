import Foundation

@MainActor final class NativeUiStyleAssets {
    private(set) var current: NativeUiRuntimeGeneration
    private var other: NativeUiRuntimeGeneration?

    init(_ installed: NativeUiRuntimeGeneration) { current = installed }

    func validate(_ candidate: NativeUiRuntimeGeneration) throws {
        try NativeUiRuntimeFailure.require(candidate.compare(current) == .prepareStyles, "Unchanged or restart-required generation cannot prepare styles")
        try sameStyles(candidate)
        try styles(current)
        try styles(candidate)
    }

    func stage(_ candidate: NativeUiRuntimeGeneration) throws {
        try validate(candidate)
        other = candidate
    }

    func promote(_ generation: String) throws {
        try NativeUiRuntimeFailure.require([current, other].compactMap { $0 }.contains { $0.descriptor.generation == generation }, "Unknown style generation")
        promoteKnown(generation)
    }

    func rollback(_ generation: String) throws {
        try restore(generation)
        other = nil
    }

    func discard() { other = nil }

    func asset(_ url: URL) throws -> Data {
        let path = try NativeUiStyleAddress.path(url)
        return try captured(path)
    }

    func stylesheetArguments(_ candidate: NativeUiRuntimeGeneration) throws -> [String: [String: String]] {
        let pairs = try candidate.descriptor.assets.filter { $0.stylesheet() }.map { record in
            (record.path, [
                "previous": try NativeUiStyleAddress.url(current.descriptor.generation, record.path).absoluteString,
                "candidate": try NativeUiStyleAddress.url(candidate.descriptor.generation, record.path).absoluteString,
            ])
        }
        return Dictionary(uniqueKeysWithValues: pairs)
    }

    private func promoteKnown(_ generation: String) {
        if current.descriptor.generation != generation, let pending = other {
            other = current
            current = pending
        }
    }

    private func restore(_ generation: String) throws {
        let previous = current.descriptor.generation == generation ? other : current
        try NativeUiRuntimeFailure.require(previous != nil, "Rollback generation is unavailable")
        current = previous ?? current
    }

    private func captured(_ path: String) throws -> Data {
        let parts = path.split(separator: "/", maxSplits: 2, omittingEmptySubsequences: false)
        let captured = generationPath(parts)
        let bytes = try captured.map { try fromGeneration($0.0, $0.1) } ?? current.asset(path)
        return try requiredBytes(bytes)
    }

    private func requiredBytes(_ bytes: Data?) throws -> Data {
        guard let bytes else { throw NativeUiRuntimeFailure(description: "Unknown captured asset") }
        return bytes
    }

    private func generationPath(_ parts: [Substring]) -> (String, String)? {
        switch (parts.first, parts.count) {
        case (".tachyon-native-ui", 3): return (String(parts[1]), String(parts[2]))
        case (".tachyon-native-ui", _): return ("", "")
        default: return nil
        }
    }

    private func fromGeneration(_ identity: String, _ path: String) throws -> Data? {
        try NativeUiRuntimeJson.digest(identity)
        try NativeUiRuntimeAsset.canonicalPath(path)
        let generation = [current, other].compactMap { $0 }.first { $0.descriptor.generation == identity }
        try NativeUiRuntimeFailure.require(generation != nil, "Unknown or retired generation")
        return generation?.asset(path)
    }

    private func sameStyles(_ candidate: NativeUiRuntimeGeneration) throws {
        let installed = current.descriptor.assets.filter { $0.stylesheet() }.map { $0.path }
        let proposed = candidate.descriptor.assets.filter { $0.stylesheet() }.map { $0.path }
        try NativeUiRuntimeFailure.require(installed == proposed && !installed.isEmpty, "Stylesheet root additions/removals need separate preparation support")
    }

    private func styles(_ generation: NativeUiRuntimeGeneration) throws {
        try generation.descriptor.assets.filter { $0.stylesheet() }.forEach { record in
            try NativeUiCssProfile.validate(generation.asset(record.path) ?? Data())
        }
        try NativeUiRuntimeFailure.require(!generation.descriptor.assets.contains { $0.path.hasPrefix(".tachyon-native-ui/") }, "Reserved native generation namespace")
    }
}
