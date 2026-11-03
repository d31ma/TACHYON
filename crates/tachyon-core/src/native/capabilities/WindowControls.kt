class WindowControls internal constructor(private val owner: NativeCapabilityOwner) {
    fun granted(control: String): Boolean = try {
        (tacParseJson(owner.answer("window.granted", mapOf("control" to control))) as? Map<*, *>)?.get("granted") == true
    } catch (_: Exception) { false }

    fun minimize() { invoke("minimize") }
    fun maximize() { invoke("maximize") }
    fun fullscreen() { invoke("fullscreen") }
    fun close() { invoke("close") }
    fun setTitle(title: String) {
        require(title.length <= 512 && !title.contains('\u0000')) { "Native window title is invalid." }
        invoke("title", mapOf("title" to title))
    }
    fun resize(width: Int, height: Int) {
        require(width in 64..16384 && height in 64..16384) { "Native window size is invalid." }
        invoke("resize", mapOf("width" to width, "height" to height))
    }

    private fun invoke(control: String, fields: Map<String, Any?> = emptyMap()) {
        check(granted(control)) { "The $control window control is not granted. Add \"$control\" to tac.application.window.controls in client/manifest.json." }
        val result = try { tacParseJson(owner.answer("window.$control", fields)) as? Map<*, *> }
            catch (_: Exception) { null }
        check(result?.get("ok") == true) { "Native window control is unavailable." }
    }
}
