@RequestRoute("/status")
@Public
class StatusController {
    @GetRoute
    fun getStatus(): String = "ok"
}
