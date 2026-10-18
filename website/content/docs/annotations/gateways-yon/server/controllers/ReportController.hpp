#pragma once
#include "../models/Report.hpp"
#include "../remotes/ReportClient.hpp"

class [[RequestRoute("/report")]] [[Public]] ReportController {
    [[Needs]] ReportClient& reportClient;

public:
    [[GetRoute]]
    Report getReport() {
        return Report{reportClient.summarize("2026-Q3"), reportClient.forecast(2)};
    }
};
