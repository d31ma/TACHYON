@request_route("/report")
class ReportController:
    @get_route
    def get_report(self, request):
        return relay(("ruby", "server/helpers/ReportDelegate.rb"), request)