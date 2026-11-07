class SecureRandom {
    fun bytes(count: Int): List<Int> =
        (TacWebHost.call("secureRandom", "bytes", listOf(count)) as? List<*> ?: emptyList<Any?>()).map { tacInt(it) }

    fun uuid(): String = tacString(TacWebHost.call("secureRandom", "uuid"))
}
