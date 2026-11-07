final class TacRunningTask {
    let id: Int
    let owner: String
    var answer: String?
    var task: Task<Void, Never>?

    init(_ id: Int, _ owner: String) {
        self.id = id
        self.owner = owner
    }

    func finish(_ work: () async throws -> Any?) async {
        do { answer = "{\"value\":" + tacEncode(try await work()) + "}" } catch { answer = failed(error) }
    }

    private func failed(_ error: Error) -> String {
        let message = "\"error\":" + tacEncode(String(describing: error))
        guard let code = Self.code(of: error) else { return "{" + message + "}" }
        return "{" + message + ",\"code\":" + tacEncode(code) + "}"
    }

    private static func code(of error: Error) -> String? {
        Mirror(reflecting: error).children.first { $0.label == "code" }?.value as? String
    }
}
