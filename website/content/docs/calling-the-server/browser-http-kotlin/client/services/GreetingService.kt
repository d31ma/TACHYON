@Service
class GreetingService {
    @Needs
    private val greetingClient: GreetingClient
    @Needs
    private val overrideClient: OverrideClient
    var message: String = ""
    var custom: String = ""
    var loads: Int = 0
    var clears: Int = 0

    suspend fun ask() {
        loads += 1
        message = greetingClient.read().message
    }

    suspend fun clear() {
        greetingClient.clear()
        clears += 1
    }

    suspend fun askCustom() {
        custom = overrideClient.read().message
    }
}
