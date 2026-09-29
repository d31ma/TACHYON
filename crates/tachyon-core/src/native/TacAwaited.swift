final class TacAwaited<Value> {
    var result: Result<Value, Error>?

    func finish(_ work: () async throws -> Value) async {
        do { result = .success(try await work()) } catch { result = .failure(error) }
    }

    func value() throws -> Value {
        guard let result else { throw TacWrongArgument.value }
        return try result.get()
    }
}
