class Clipboard {
    fun writeText(text: String) {
        TacWebHost.call("clipboard", "writeText", listOf(text))
    }

    fun readText() {
        TacWebHost.call("clipboard", "readText")
    }
}
