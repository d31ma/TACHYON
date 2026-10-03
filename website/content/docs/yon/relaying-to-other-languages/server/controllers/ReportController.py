@request_route("/report")
@public
class ReportController:
    @get_route
    def get_report(self, request: object) -> object:
        return relay(("perl", "server/relays/report.pl"), request)
