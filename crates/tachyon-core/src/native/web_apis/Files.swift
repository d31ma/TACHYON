final class Files {
    func pick(accept: String, multiple: Bool) throws {
        _ = try TacWebHost.call("files", "pick", [accept, multiple])
    }

    func read(id: String, offset: Int, count: Int) throws {
        _ = try TacWebHost.call("files", "read", [id, offset, count])
    }

    func release(id: String) throws {
        _ = try TacWebHost.call("files", "release", [id])
    }

    func upload(id: String, method: String, url: String, headers: String) throws -> String {
        try TacWebHost.answer(TacWebHost.call("files", "upload", [id, method, url, headers]), as: String.self)
    }

    func cancel(upload: String) throws {
        _ = try TacWebHost.call("files", "cancel", [upload])
    }
}
