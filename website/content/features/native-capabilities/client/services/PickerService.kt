@Service
@NeedsAll
class PickerService {
    private val filePicker: FilePicker
    var status: String = "Ready"

    fun choose() {
        val chosen = filePicker.open(FilePickerOptions(types = listOf("text/plain")))
        status = readSelected(chosen)
    }

    private fun readSelected(chosen: FilePickerResult): String {
        val handle = chosen.handle ?: return chosen.status.wire
        val content = filePicker.readText(handle)
        filePicker.release(handle)
        return content.status.wire
    }
}
