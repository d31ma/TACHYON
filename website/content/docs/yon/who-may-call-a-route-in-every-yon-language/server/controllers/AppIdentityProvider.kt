import com.sun.net.httpserver.HttpExchange
import models.Identity

@IdentityProvider
class AppIdentityProvider {
    fun identify(exchange: HttpExchange): Identity? {
        return find(exchange.requestHeaders.getFirst("Authorization").orEmpty())
    }

    private fun find(token: String): Identity? {
        return SESSIONS[token]
    }

    companion object {
        val SESSIONS = mapOf(
            "ada-token" to Identity("ada", listOf("Admin")),
            "bob-token" to Identity("bob", listOf()),
        )
    }
}
