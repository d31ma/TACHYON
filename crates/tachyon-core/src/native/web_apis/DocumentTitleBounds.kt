private object DocumentTitleBounds {
    private const val MOST_BYTES = 1024

    fun check(title: String) {
        require(!title.contains('\u0000')) { "DocumentTitle rejects NUL." }
        unicode(title)
        require(title.encodeToByteArray().size <= MOST_BYTES) { "DocumentTitle exceeds 1024 UTF-8 bytes." }
    }

    private fun unicode(title: String) {
        var index = 0
        while (index < title.length) index += width(title, index)
    }

    private fun width(title: String, index: Int): Int {
        val value = title[index]
        if (value in '\uD800'..'\uDBFF') return pair(title, index)
        require(value !in '\uDC00'..'\uDFFF') { "DocumentTitle needs valid Unicode." }
        return 1
    }

    private fun pair(title: String, index: Int): Int {
        require(index + 1 < title.length && title[index + 1] in '\uDC00'..'\uDFFF') { "DocumentTitle needs valid Unicode." }
        return 2
    }
}
