object PageCapabilityTransport {
    private const val UNSUPPORTED = "{\"status\":\"unsupported\"}"

    fun send(capability: String, request: String): String =
        try { TacWebHost.call("nativeCapability", "call", listOf("kotlin", capability, request)) as? String ?: UNSUPPORTED }
        catch (_: Exception) { UNSUPPORTED }
}
