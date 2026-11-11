class Files {
    fun pick(accept: String, multiple: Boolean) {
        TacWebHost.call("files", "pick", listOf(accept, multiple))
    }

    fun read(id: String, offset: Long, count: Int) {
        TacWebHost.call("files", "read", listOf(id, offset, count))
    }

    fun release(id: String) {
        TacWebHost.call("files", "release", listOf(id))
    }

    fun upload(id: String, method: String, url: String, headers: String): String =
        tacString(TacWebHost.call("files", "upload", listOf(id, method, url, headers)))

    fun cancel(upload: String) {
        TacWebHost.call("files", "cancel", listOf(upload))
    }
}
