class StorageSpace {
    fun estimate() {
        TacWebHost.call("storageSpace", "estimate")
    }

    fun persist() {
        TacWebHost.call("storageSpace", "persist")
    }
}
