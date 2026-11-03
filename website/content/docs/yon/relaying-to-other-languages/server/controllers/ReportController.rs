#[RequestRoute("/report")]
#[Public]
struct ReportController {}
impl ReportController {
    fn new() -> Self { Self {} }

    #[GetRoute]
    fn get_report(&self, request: &http::Request<Vec<u8>>) -> http::Response<Vec<u8>> {
        relay(&["perl", "server/relays/report.pl"], request)
    }
}
