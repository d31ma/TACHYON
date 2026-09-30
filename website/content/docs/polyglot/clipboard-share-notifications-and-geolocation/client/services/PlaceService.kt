@Service
class PlaceService {
    private val geolocation = Geolocation()
    private val clipboard = Clipboard()
    var place: String = ""
    var problem: String = ""

    fun locate() {
        keepProblem { geolocation.currentPosition() }
    }

    fun copy() {
        keepProblem { clipboard.writeText(place) }
    }

    @OnEvent
    fun found(found: PositionFound) {
        place = "${found.latitude}, ${found.longitude}"
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
}
