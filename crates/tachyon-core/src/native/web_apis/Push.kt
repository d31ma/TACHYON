class Push {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("push", "supported"))

    fun subscribe(applicationServerKey: String) {
        TacWebHost.call("push", "subscribe", listOf(applicationServerKey))
    }

    fun subscription() {
        TacWebHost.call("push", "subscription")
    }

    fun unsubscribe() {
        TacWebHost.call("push", "unsubscribe")
    }
}
