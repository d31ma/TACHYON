class WindowControls {
    private val api: String = "windowControls"

    fun granted(control: String): Boolean = tacBoolean(TacWebHost.call(api, "granted", listOf(control)))

    fun minimize() {
        TacWebHost.call(api, "minimize")
    }

    fun maximize() {
        TacWebHost.call(api, "maximize")
    }

    fun fullscreen() {
        TacWebHost.call(api, "fullscreen")
    }

    fun close() {
        TacWebHost.call(api, "close")
    }

    fun resize(width: Int, height: Int) {
        TacWebHost.call(api, "resize", listOf(width, height))
    }

    fun setTitle(text: String) {
        TacWebHost.call(api, "setTitle", listOf(text))
    }
}
