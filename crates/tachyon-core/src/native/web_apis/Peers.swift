final class Peers {
    private static let api = "peers"

    func supported() throws -> Bool {
        try TacWebHost.flag(Self.api, "supported")
    }

    func open(username: String, credential: String) throws -> String {
        try TacWebHost.answer(TacWebHost.call(Self.api, "open", [username, credential]), as: String.self)
    }

    func offer(connection: String) throws {
        _ = try TacWebHost.call(Self.api, "offer", [connection])
    }

    func answer(connection: String, offerSdp: String) throws {
        _ = try TacWebHost.call(Self.api, "answer", [connection, offerSdp])
    }

    func accept(connection: String, answerSdp: String) throws {
        _ = try TacWebHost.call(Self.api, "accept", [connection, answerSdp])
    }

    func addCandidate(connection: String, candidateJson: String) throws {
        _ = try TacWebHost.call(Self.api, "addCandidate", [connection, candidateJson])
    }

    func restart(connection: String) throws {
        _ = try TacWebHost.call(Self.api, "restart", [connection])
    }

    func openChannel(connection: String, label: String, ordered: Bool) throws -> String {
        try TacWebHost.answer(TacWebHost.call(Self.api, "openChannel", [connection, label, ordered]), as: String.self)
    }

    func send(connection: String, channel: String, data: String) throws -> Bool {
        try TacWebHost.answer(TacWebHost.call(Self.api, "send", [connection, channel, data]), as: Bool.self)
    }

    func closeChannel(connection: String, channel: String) throws {
        _ = try TacWebHost.call(Self.api, "closeChannel", [connection, channel])
    }

    func close(connection: String) throws {
        _ = try TacWebHost.call(Self.api, "close", [connection])
    }
}
