@Service
class ReportService {
    private val windowControls = WindowControls()
    var problem: String = ""

    fun minimize() {
        keepProblem { windowControls.minimize() }
    }

    fun resize() {
        keepProblem { windowControls.resize(1280, 800) }
    }

    fun rename() {
        keepProblem { windowControls.setTitle("Quarterly report") }
    }

    private fun keepProblem(call: () -> Unit) {
        try {
            call()
        } catch (refusal: IllegalStateException) {
            problem = refusal.message.orEmpty()
        }
    }
}
