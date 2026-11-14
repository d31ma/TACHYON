final class Blobs {
    func saveHeld(fileId: String, key: String) throws {
        _ = try TacWebHost.call("blobs", "saveHeld", [fileId, key])
    }

    func download(url: String, key: String, headers: String) throws {
        _ = try TacWebHost.call("blobs", "download", [url, key, headers])
    }

    func list(prefix: String) throws {
        _ = try TacWebHost.call("blobs", "list", [prefix])
    }

    func read(key: String, offset: Int, count: Int) throws {
        _ = try TacWebHost.call("blobs", "read", [key, offset, count])
    }

    func remove(key: String) throws {
        _ = try TacWebHost.call("blobs", "remove", [key])
    }

    func url(key: String) throws -> String {
        try TacWebHost.answer(TacWebHost.call("blobs", "url", [key]), as: String.self)
    }

    func open(key: String) throws {
        _ = try TacWebHost.call("blobs", "open", [key])
    }

    func close(key: String) throws {
        _ = try TacWebHost.call("blobs", "close", [key])
    }

    func upload(key: String, method: String, url: String, headers: String) throws -> String {
        try TacWebHost.answer(TacWebHost.call("blobs", "upload", [key, method, url, headers]), as: String.self)
    }

    func cancel(upload: String) throws {
        _ = try TacWebHost.call("blobs", "cancel", [upload])
    }
}
