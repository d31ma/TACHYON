import models.ProgramState

@Service
class WriterService {
    @Needs
    private val pageProgramClient: PageProgramClient
    @Needs
    private val programNoteStore: ProgramNoteStore
    var identity: String = ""
    var count: Int = 0
    var recalled: String = ""

    suspend fun inspect() {
        show(pageProgramClient.read())
    }

    suspend fun increment() {
        show(pageProgramClient.increment())
    }

    fun remember(text: String) {
        programNoteStore.save(mapOf(ID to NOTE, TEXT to text))
    }

    fun recall() {
        recalled = textOf(programNoteStore.findById(NOTE))
    }

    private fun show(state: ProgramState) {
        identity = state.identity
        count = state.count
    }

    private fun textOf(note: Any?): String = (note as? Map<*, *>)?.get(TEXT)?.toString().orEmpty()

    companion object {
        const val ID = "id"
        const val NOTE = "note"
        const val TEXT = "text"
    }
}
