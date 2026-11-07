class Share {
    fun open(title: String, text: String, url: String) {
        TacWebHost.call("share", "open", listOf(title, text, url))
    }
}
