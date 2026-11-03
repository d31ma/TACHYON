fun interface TacNativeCapabilityCall {
    fun call(capability: String, payload: String): String
}
