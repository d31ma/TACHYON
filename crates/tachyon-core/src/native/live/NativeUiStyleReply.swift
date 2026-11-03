import Foundation

@MainActor final class NativeUiStyleReply {
    private var continuation: CheckedContinuation<String, Error>?
    private var timer: DispatchWorkItem?

    init(_ continuation: CheckedContinuation<String, Error>) { self.continuation = continuation }

    func deadline() {
        let timer = DispatchWorkItem { [weak self] in self?.finish(.failure(NativeUiRuntimeFailure(description: "Native stylesheet acknowledgement deadline exceeded"))) }
        self.timer = timer
        DispatchQueue.main.asyncAfter(deadline: .now() + 12, execute: timer)
    }

    func resolve(_ result: Result<Any, Error>) {
        let typed = result.flatMap(Self.string)
        finish(typed)
    }

    private func finish(_ result: Result<String, Error>) {
        guard let continuation else { return }
        self.continuation = nil
        timer?.cancel()
        timer = nil
        continuation.resume(with: result)
    }

    private static func string(_ value: Any) -> Result<String, Error> {
        guard let text = value as? String, text.utf8.count <= 128 else {
            return .failure(NativeUiRuntimeFailure(description: "Invalid stylesheet acknowledgement"))
        }
        return .success(text)
    }
}
