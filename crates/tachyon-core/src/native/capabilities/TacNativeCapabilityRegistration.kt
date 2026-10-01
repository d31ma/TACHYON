object TacNativeCapabilityRegistration {
    @JvmStatic
    fun register(adapter: TacNativeCapabilityCall) {
        NativeCapabilities.call = { capability, payload -> adapter.call(capability, payload) }
    }
}
