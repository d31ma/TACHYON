class Connectivity {
    fun online(): Boolean = tacBoolean(TacWebHost.call("connectivity", "online"))
}
