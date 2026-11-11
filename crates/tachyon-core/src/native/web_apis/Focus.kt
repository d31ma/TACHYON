class Focus {
    fun move(target: String) {
        TacWebHost.call("focus", "move", listOf(target))
    }

    fun trap(container: String) {
        TacWebHost.call("focus", "trap", listOf(container))
    }

    fun release() {
        TacWebHost.call("focus", "release")
    }
}
