import com.sun.net.httpserver.HttpExchange;

@RequestRoute("/report")
@Public
class ReportController {
    @GetRoute
    void getReport(HttpExchange exchange) throws java.io.IOException {
        Yon.relay(java.util.List.of("ruby", "server/helpers/ReportDelegate.rb"), exchange);
    }
}
