class FilePicker internal constructor(private val owner: NativeCapabilityOwner) {
    fun available(): Boolean = answer("available", emptyMap()).status == FilePickerStatus.AVAILABLE

    fun open(options: FilePickerOptions = FilePickerOptions()): FilePickerResult {
        if (options.multiple || options.types.any { it != "text/plain" && it != "application/json" }) return FilePickerResult(FilePickerStatus.INVALID)
        return answer("open", mapOf("types" to options.types))
    }

    fun readText(handle: String): FilePickerResult = answer("readText", mapOf("handle" to handle))
    fun release(handle: String): FilePickerResult = answer("release", mapOf("handle" to handle))
    private fun answer(operation: String, fields: Map<String, Any?>): FilePickerResult = FilePickerResult.decode(owner.answer("filePicker.$operation", fields))
}
