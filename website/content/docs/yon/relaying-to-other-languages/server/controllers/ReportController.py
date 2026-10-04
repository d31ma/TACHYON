@request_route("/report")
@public
class ReportController:
    @get_route
    def get_report(self, request: object) -> object:
        return relay(("ruby", "server/helpers/ReportDelegate.rb"), request)
