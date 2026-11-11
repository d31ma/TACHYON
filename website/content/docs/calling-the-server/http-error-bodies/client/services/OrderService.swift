@Service
final class OrderService {
    @Needs
    private let orderClient: OrderClient
    var status: Int = 0
    var title: String = ""
    var mistakes: Int = 0

    func send() async {
        await keepProblem { _ = try await orderClient.send(order: .object(["name": .string(""), "level": .number(12)])) }
    }

    private func keepProblem(_ call: () async throws -> Void) async {
        do {
            try await call()
        } catch {
            show(error)
        }
    }

    private func show(_ error: Error) {
        status = Behaviors.errorStatus(error)
        guard let problem = Behaviors.errorBody(error, as: Problem.self) else { return }
        title = problem.title
        mistakes = problem.errors.count
    }
}
