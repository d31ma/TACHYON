import Foundation

final class NativeUiRuntimeGeneration {
    let descriptor: NativeUiRuntimeDescriptor
    private let assets: [String: Data]

    private init(_ descriptor: NativeUiRuntimeDescriptor, _ assets: [String: Data]) {
        self.descriptor = descriptor
        self.assets = assets
    }

    static func trusted(_ directory: Int32, _ source: Data, _ fingerprint: String) throws -> NativeUiRuntimeGeneration {
        let descriptor = try NativeUiRuntimeDescriptor.trusted(source, fingerprint)
        let assets = try NativeUiRuntimeCapture.read(directory)
        try matches(descriptor, assets)
        return NativeUiRuntimeGeneration(descriptor, assets)
    }

    func asset(_ path: String) -> Data? { assets[path] }

    func compare(_ installed: NativeUiRuntimeGeneration) -> NativeUiRuntimeChange { comparison(installed) }

    private func comparison(_ installed: NativeUiRuntimeGeneration) -> NativeUiRuntimeChange {
        let unchanged = descriptor == installed.descriptor
        return unchanged ? .unchanged : changed(installed)
    }

    private func changed(_ installed: NativeUiRuntimeGeneration) -> NativeUiRuntimeChange {
        let fixed = descriptor.assets.filter { !$0.stylesheet() }
        let previous = installed.descriptor.assets.filter { !$0.stylesheet() }
        let compatible = descriptor.contract == installed.descriptor.contract && fixed == previous
        return compatible ? .prepareStyles : .restartRequired
    }

    private static func matches(_ descriptor: NativeUiRuntimeDescriptor, _ assets: [String: Data]) throws {
        try NativeUiRuntimeFailure.require(descriptor.assets.count == assets.count, "Incomplete captured generation")
        try descriptor.assets.forEach { try match($0, assets) }
    }

    private static func match(_ record: NativeUiRuntimeAsset, _ assets: [String: Data]) throws {
        guard let bytes = assets[record.path] else { throw NativeUiRuntimeFailure(description: "Missing captured asset") }
        try NativeUiRuntimeFailure.require(UInt64(bytes.count) == record.size, "Captured asset size differs")
        try NativeUiRuntimeFailure.require(NativeUiRuntimeJson.sha256(bytes) == record.sha256, "Captured asset hash differs")
    }
}
