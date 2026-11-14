final class CodeScanner {
    private static let api = "codeScanner"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func scan() throws {
        _ = try TacWebHost.call(Self.api, "scan")
    }

    func cancel() throws {
        _ = try TacWebHost.call(Self.api, "cancel")
    }
}
