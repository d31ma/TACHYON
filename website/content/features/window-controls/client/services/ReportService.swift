@Service
final class ReportService {
    private let windowControls = WindowControls()
    var problem: String = ""

    func minimize() {
        keepProblem { try windowControls.minimize() }
    }

    func resize() {
        keepProblem { try windowControls.resize(width: 1280, height: 800) }
    }

    func rename() {
        keepProblem { try windowControls.setTitle("Quarterly report") }
    }

    private func keepProblem(_ call: () throws -> Void) {
        do {
            try call()
        } catch {
            problem = String(describing: error)
        }
    }
}
