@Service
class RenameService {
    private val focus = Focus()
    var editing: Boolean = false
    var problem: String = ""

    fun open() {
        editing = true
        keepProblem { focus.trap("dialog") }
        keepProblem { focus.move("name") }
    }

    fun close() {
        editing = false
        keepProblem { focus.release() }
    }

    @OnEvent
    fun failed(failed: BrowserCallFailed) {
        problem = failed.message
    }

    private fun keepProblem(call: () -> Unit) {
        try {
            call()
        } catch (refusal: IllegalStateException) {
            problem = refusal.message.orEmpty()
        }
    }
}
