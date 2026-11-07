import Foundation

struct NativeUiRuntimeDescriptor: Decodable, Equatable {
    let schemaVersion: UInt8
    let contract: NativeUiRuntimeContract
    let assets: [NativeUiRuntimeAsset]
    let generation: String

    static func trusted(_ source: Data, _ fingerprint: String) throws -> Self {
        try bounded(source)
        try NativeUiRuntimeJson.digest(fingerprint)
        try trustedFingerprint(source, fingerprint)
        let descriptor = try JSONDecoder().decode(Self.self, from: source)
        try descriptor.validate()
        try NativeUiRuntimeFailure.require(source == descriptor.canonical(), "Descriptor must use canonical compiler encoding")
        return descriptor
    }

    func canonical() -> Data {
        let value = "{\"schemaVersion\":" + String(schemaVersion)
            + ",\"contract\":" + contract.canonical()
            + ",\"assets\":" + assetRecords()
            + ",\"generation\":" + NativeUiRuntimeJson.quote(generation) + "}"
        return NativeUiRuntimeJson.bytes(value)
    }

    private func validate() throws {
        try contract.validate()
        try Self.schema(schemaVersion)
        try NativeUiRuntimeJson.digest(generation)
        try assets.forEach { try $0.validate() }
        try boundedAssets()
        try orderedPaths()
        try generationIdentity()
    }

    private func boundedAssets() throws {
        try NativeUiRuntimeFailure.require((1...NativeUiRuntimeBounds.files).contains(assets.count), "Invalid asset count")
        try NativeUiRuntimeFailure.require(assets.reduce(UInt64(0)) { $0 + $1.size } <= UInt64(NativeUiRuntimeBounds.totalBytes), "Capture byte limit exceeded")
    }

    private func orderedPaths() throws {
        let paths = assets.map { Data($0.path.utf8) }
        let sorted = paths.sorted { $0.lexicographicallyPrecedes($1) }
        try NativeUiRuntimeFailure.require(paths == sorted && Set(paths).count == paths.count, "Asset paths must be unique and ordered")
    }

    private func generationIdentity() throws {
        let identity = NativeUiRuntimeJson.bytes("[" + contract.canonical() + "," + assetRecords() + "]")
        try NativeUiRuntimeFailure.require(NativeUiRuntimeJson.sha256(identity) == generation, "Invalid generation identity")
    }

    private func assetRecords() -> String { "[" + assets.map { $0.canonical() }.joined(separator: ",") + "]" }

    private static func bounded(_ source: Data) throws {
        try NativeUiRuntimeFailure.require(source.count <= NativeUiRuntimeBounds.descriptorBytes, "Descriptor byte limit exceeded")
    }

    private static func trustedFingerprint(_ source: Data, _ fingerprint: String) throws {
        try NativeUiRuntimeFailure.require(NativeUiRuntimeJson.sha256(source) == fingerprint, "Descriptor is not independently trusted")
    }

    private static func schema(_ version: UInt8) throws {
        try NativeUiRuntimeFailure.require(version == 1, "Unknown descriptor schema")
    }
}
