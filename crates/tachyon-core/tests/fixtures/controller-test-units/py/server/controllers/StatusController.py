@request_route("/status")
@public
class StatusController:
    @get_route
    def get_status(self) -> str:
        return "ok"
