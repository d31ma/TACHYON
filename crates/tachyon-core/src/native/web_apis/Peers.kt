class Peers {
    fun supported(): Boolean = tacBoolean(TacWebHost.call("peers", "supported"))

    fun open(username: String, credential: String): String =
        tacString(TacWebHost.call("peers", "open", listOf(username, credential)))

    fun offer(connection: String) {
        TacWebHost.call("peers", "offer", listOf(connection))
    }

    fun answer(connection: String, offerSdp: String) {
        TacWebHost.call("peers", "answer", listOf(connection, offerSdp))
    }

    fun accept(connection: String, answerSdp: String) {
        TacWebHost.call("peers", "accept", listOf(connection, answerSdp))
    }

    fun addCandidate(connection: String, candidateJson: String) {
        TacWebHost.call("peers", "addCandidate", listOf(connection, candidateJson))
    }

    fun restart(connection: String) {
        TacWebHost.call("peers", "restart", listOf(connection))
    }

    fun openChannel(connection: String, label: String, ordered: Boolean): String =
        tacString(TacWebHost.call("peers", "openChannel", listOf(connection, label, ordered)))

    fun send(connection: String, channel: String, data: String): Boolean =
        tacBoolean(TacWebHost.call("peers", "send", listOf(connection, channel, data)))

    fun closeChannel(connection: String, channel: String) {
        TacWebHost.call("peers", "closeChannel", listOf(connection, channel))
    }

    fun close(connection: String) {
        TacWebHost.call("peers", "close", listOf(connection))
    }
}
