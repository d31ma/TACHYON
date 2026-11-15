class Passkeys {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("passkeys", "supported"))

    fun check() {
        TacWebHost.call("passkeys", "check")
    }

    fun create(optionsJson: String, prfKey: String) {
        TacWebHost.call("passkeys", "create", listOf(optionsJson, prfKey))
    }

    fun get(optionsJson: String, prfKey: String) {
        TacWebHost.call("passkeys", "get", listOf(optionsJson, prfKey))
    }

    fun offer(optionsJson: String, prfKey: String) {
        TacWebHost.call("passkeys", "offer", listOf(optionsJson, prfKey))
    }

    fun cancel() {
        TacWebHost.call("passkeys", "cancel")
    }
}
