private class TacPageScope(send: (String) -> Unit) {
    private var disposed = false
    val documentTitle: DocumentTitle = BrowserDocumentTitle(this, send)

    fun check() {
        check(!disposed) { "DocumentTitle owner is disposed." }
    }

    fun dispose() {
        disposed = true
    }
}
