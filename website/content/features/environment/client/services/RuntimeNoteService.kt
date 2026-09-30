@Service
class RuntimeNoteService {
    val runtime: String = "Kotlin, compiled to WebAssembly or into the APK"

    fun showsNotes(development: Boolean): Boolean = development
}
