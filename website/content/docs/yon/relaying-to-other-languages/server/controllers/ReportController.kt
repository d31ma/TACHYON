import com.sun.net.httpserver.HttpExchange

@RequestRoute("/report")
@Public
class ReportController {
    @GetRoute
    fun getReport(exchange: HttpExchange) =
        Yon.relay(listOf("perl", "server/relays/report.pl"), exchange)
}
