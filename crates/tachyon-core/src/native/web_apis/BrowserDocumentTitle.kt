private class BrowserDocumentTitle(private val scope: TacPageScope, private val send: (String) -> Unit) : DocumentTitle {
    override fun set(title: String) {
        scope.check()
        DocumentTitleBounds.check(title)
        send(title)
    }
}
