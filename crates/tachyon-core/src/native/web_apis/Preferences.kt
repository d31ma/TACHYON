class Preferences {
    fun scheme(): String = text("scheme")

    fun motion(): String = text("motion")

    fun language(): String = text("language")

    private fun text(call: String): String = tacString(TacWebHost.call("preferences", call))
}
