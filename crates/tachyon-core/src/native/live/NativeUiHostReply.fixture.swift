import Foundation

@MainActor final class NativeUiHostReplyFixture {
    private static let shared = NativeUiHostReplyFixture()
    private var pending: (String, (Any?, String?) -> Void)?
    private var timer: DispatchWorkItem?

    static func deliver(_ answer: String, _ request: [String: Any], _ reply: @escaping (Any?, String?) -> Void) {
        shared.delivered(answer, request, reply)
    }

    static func active() -> Bool { shared.pending != nil }
    static func release() throws { try shared.released() }

    private func delivered(_ answer: String, _ request: [String: Any], _ reply: @escaping (Any?, String?) -> Void) {
        switch request["fixtureDelay"] as? Bool {
        case true: held(answer, reply)
        default: reply(answer, nil)
        }
    }

    private func held(_ answer: String, _ reply: @escaping (Any?, String?) -> Void) {
        guard pending == nil else { reply(nil, "Fixture already holds a reply"); return }
        pending = (answer, reply)
        let timer = DispatchWorkItem { [weak self] in self?.expired() }
        self.timer = timer
        DispatchQueue.main.asyncAfter(deadline: .now() + 10, execute: timer)
    }

    private func released() throws {
        guard let (answer, reply) = take() else { throw NativeUiRuntimeFailure(description: "No actual native reply was pending") }
        reply(answer, nil)
    }

    private func expired() {
        guard let reply = take() else { return }
        reply.1(nil, "Native reply fixture deadline exceeded")
    }

    private func take() -> (String, (Any?, String?) -> Void)? {
        let reply = pending
        pending = nil
        timer?.cancel()
        timer = nil
        return reply
    }
}
