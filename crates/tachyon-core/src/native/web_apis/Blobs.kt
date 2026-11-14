class Blobs {
    fun saveHeld(fileId: String, key: String) {
        TacWebHost.call("blobs", "saveHeld", listOf(fileId, key))
    }

    fun download(url: String, key: String, headers: String) {
        TacWebHost.call("blobs", "download", listOf(url, key, headers))
    }

    fun list(prefix: String) {
        TacWebHost.call("blobs", "list", listOf(prefix))
    }

    fun read(key: String, offset: Long, count: Int) {
        TacWebHost.call("blobs", "read", listOf(key, offset, count))
    }

    fun remove(key: String) {
        TacWebHost.call("blobs", "remove", listOf(key))
    }

    fun url(key: String): String = tacString(TacWebHost.call("blobs", "url", listOf(key)))

    fun open(key: String) {
        TacWebHost.call("blobs", "open", listOf(key))
    }

    fun close(key: String) {
        TacWebHost.call("blobs", "close", listOf(key))
    }

    fun upload(key: String, method: String, url: String, headers: String): String =
        tacString(TacWebHost.call("blobs", "upload", listOf(key, method, url, headers)))

    fun cancel(upload: String) {
        TacWebHost.call("blobs", "cancel", listOf(upload))
    }
}
