@Service
@Lifetime({"value":"prototype"})
@NeedsAll
class KotlinTitleService {
    private val documentTitle: DocumentTitle
    val label: String
    val fails: Boolean

    @OnStart
    fun initialize() {
        documentTitle.set(label)
        check(!fails) { "DocumentTitle fixture startup refused." }
    }

    fun write(title: String) {
        documentTitle.set(title)
    }

    fun attempt(title: String): Boolean = accepted(title)

    private fun accepted(title: String): Boolean {
        return try { documentTitle.set(title); true }
        catch (_: Exception) { false }
    }
}
