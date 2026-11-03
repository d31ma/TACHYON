class Notifications {
    fun permission(): String = tacString(TacWebHost.call("notifications", "permission"))

    fun requestPermission() {
        TacWebHost.call("notifications", "requestPermission")
    }

    fun show(title: String, body: String) {
        TacWebHost.call("notifications", "show", listOf(title, body))
    }
}
