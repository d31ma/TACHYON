object NativeCapabilities {
    var call: (String, String) -> String = { _, _ -> "{\"status\":\"unsupported\"}" }
    private var constructing: NativeCapabilityOwner? = null
    private val owners = linkedMapOf<String, NativeCapabilityOwner>()

    fun <T> create(owner: String, work: () -> T): T {
        val previous = constructing
        dispose(owner)
        require(owners.size < 256) { "Too many native capability activations." }
        val activation = NativeCapabilityOwner(owner, call)
        owners[owner] = activation
        constructing = activation
        return try { work() } catch (error: Exception) { dispose(owner); throw error }
            finally { constructing = previous }
    }

    fun provider(): NativeCapabilityOwner = constructing ?: NativeCapabilityOwner("", call)

    fun dispose(owner: String) { owners.remove(owner)?.dispose() }
}
