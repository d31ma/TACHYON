import com.sun.net.httpserver.HttpExchange
import models.Identity

@IdentityProvider
class AppIdentityProvider {
    fun identify(exchange: HttpExchange): Identity? {
        val token = exchange.requestHeaders.getFirst("Authorization").orEmpty()
        refuseBroken(token)
        return find(token)
    }

    private fun refuseBroken(token: String) {
        if (token == BROKEN_TOKEN) throw IllegalStateException("the session store is down")
    }

    private fun find(token: String): Identity? {
        return SESSIONS[token]
    }

    companion object {
        val SESSIONS = mapOf(
            "ada-token" to Identity("ada", listOf("Admin")),
            "bob-token" to Identity("bob", listOf()),
        )
        const val BROKEN_TOKEN = "broken-token"
    }
}
