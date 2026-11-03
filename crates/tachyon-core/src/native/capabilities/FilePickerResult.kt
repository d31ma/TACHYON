data class FilePickerResult(val status: FilePickerStatus, val handle: String? = null, val name: String? = null, val text: String? = null) {
    companion object {
        internal fun decode(text: String): FilePickerResult {
            return try {
                if (text.length > 786432) return FilePickerResult(FilePickerStatus.UNAVAILABLE)
                val value = tacParseJson(text) as? Map<*, *> ?: return FilePickerResult(FilePickerStatus.UNAVAILABLE)
                val status = FilePickerStatus.entries.firstOrNull { it.wire == value["status"] } ?: return FilePickerResult(FilePickerStatus.UNAVAILABLE)
                if (status == FilePickerStatus.SELECTED) {
                    val handle = value["handle"] as? String
                    val name = value["name"] as? String
                    if (handle.isNullOrEmpty() || handle.length > 256 || handle.any { it == '/' || it == '\\' || it == '\u0000' } || name == null || name.length > 512 || name.any { it == '/' || it == '\\' || it == '\u0000' }) return FilePickerResult(FilePickerStatus.UNAVAILABLE)
                    FilePickerResult(status, handle, name)
                } else if (status == FilePickerStatus.READ) {
                    val content = value["text"] as? String
                    if (content == null || content.encodeToByteArray().size > 65536) FilePickerResult(FilePickerStatus.UNAVAILABLE) else FilePickerResult(status, text = content)
                } else FilePickerResult(status)
            } catch (_: Exception) { FilePickerResult(FilePickerStatus.UNAVAILABLE) }
        }
    }
}
