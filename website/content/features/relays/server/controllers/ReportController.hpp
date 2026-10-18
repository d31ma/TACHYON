#pragma once

class [[RequestRoute("/report")]] [[Public]] ReportController {
public:
    [[GetRoute]]
    tachyon::http::HttpResponseMessage getReport(const tachyon::http::HttpRequestMessage& request) {
        return relay({"perl", "server/relays/report.pl"}, request);
    }
};
