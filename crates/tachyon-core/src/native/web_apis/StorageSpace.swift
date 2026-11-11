final class StorageSpace {
    private static let api = "storageSpace"

    func estimate() throws {
        _ = try TacWebHost.call(Self.api, "estimate")
    }

    func persist() throws {
        _ = try TacWebHost.call(Self.api, "persist")
    }
}
