class Locks {
    fun request(name: String) {
        TacWebHost.call("locks", "request", listOf(name))
    }

    fun release(name: String) {
        TacWebHost.call("locks", "release", listOf(name))
    }
}
