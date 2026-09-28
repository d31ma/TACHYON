class PageVisibility {
    fun visible(): Boolean = tacBoolean(TacWebHost.call("pageVisibility", "visible"))
}
