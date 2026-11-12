class Passkeys {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("passkeys", "supported"))

    fun check() {
        TacWebHost.call("passkeys", "check")
    }

    fun create(optionsJson: String) {
        TacWebHost.call("passkeys", "create", listOf(optionsJson))
    }

    fun get(optionsJson: String) {
        TacWebHost.call("passkeys", "get", listOf(optionsJson))
    }

    fun offer(optionsJson: String) {
        TacWebHost.call("passkeys", "offer", listOf(optionsJson))
    }

    fun cancel() {
        TacWebHost.call("passkeys", "cancel")
    }
}
