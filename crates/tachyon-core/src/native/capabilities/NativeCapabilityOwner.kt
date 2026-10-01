class NativeCapabilityOwner internal constructor(private val owner: String, private val call: (String, String) -> String) {
    private var active = true

    internal fun answer(capability: String, fields: Map<String, Any?>): String {
        if (owner.isEmpty()) return "{\"status\":\"unsupported\"}"
        if (!active) return "{\"status\":\"disposed\"}"
        if (!tacOnOwnerThread()) return "{\"status\":\"unavailable\"}"
        return try {
            val request = tacEncode(fields + ("owner" to owner))
            if (capability.encodeToByteArray().size > 64 || request.encodeToByteArray().size > 65536) "{\"status\":\"invalid\"}" else call(capability, request)
        }
            catch (_: Exception) { "{\"status\":\"unavailable\"}" }
    }

    internal fun dispose() {
        if (!active) return
        answer("filePicker.dispose", emptyMap())
        active = false
    }
}
