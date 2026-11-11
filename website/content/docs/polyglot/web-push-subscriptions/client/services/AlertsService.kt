@Service
class AlertsService {
    private val push = Push()
    var endpoint: String = ""
    var problem: String = ""

    fun turnOn() {
        keepProblem { push.subscribe(SERVER_KEY) }
    }

    fun turnOff() {
        keepProblem { push.unsubscribe() }
    }

    @OnEvent
    fun subscribed(subscribed: PushSubscribed) {
        endpoint = subscribed.endpoint
    }

    @OnEvent
    fun unsubscribed(unsubscribed: PushUnsubscribed) {
        endpoint = ""
    }

    @OnEvent
    fun failed(failed: BrowserCallFailed) {
        problem = failed.message
    }

    private fun keepProblem(call: () -> Unit) {
        try {
            call()
        } catch (refusal: IllegalStateException) {
            problem = refusal.message.orEmpty()
        }
    }

    companion object {
        const val SERVER_KEY = "BEl62iUYgUivxIkv69yViEuiBIa-Ib9-SkvMeAtA3LFgDzkrxZJjSgSnfckjBJuBkr3qBUYIHBQFLXYp5Nksh8U"
    }
}
