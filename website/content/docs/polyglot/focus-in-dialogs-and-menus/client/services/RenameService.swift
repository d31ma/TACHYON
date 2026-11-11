@Service
final class RenameService {
    private let focus = Focus()
    var editing: Bool = false
    var problem: String = ""

    func open() {
        editing = true
        keepProblem { try focus.trap("dialog") }
        keepProblem { try focus.move("name") }
    }

    func close() {
        editing = false
        keepProblem { try focus.release() }
    }

    @OnEvent
    func failed(_ failed: BrowserCallFailed) {
        problem = failed.message
    }

    private func keepProblem(_ call: () throws -> Void) {
        do {
            try call()
        } catch {
            problem = String(describing: error)
        }
    }
}
