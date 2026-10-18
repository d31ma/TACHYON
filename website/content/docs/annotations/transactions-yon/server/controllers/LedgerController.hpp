#pragma once
#include "../models/LedgerReport.hpp"

class [[RequestRoute("/ledger")]] [[Public]] LedgerController {
    [[Needs]] LedgerService& ledger;

public:
    [[GetRoute]]
    LedgerReport getLedger() {
        LedgerReport report = ledger.settle("open");
        report.commands = LedgerSetup::commands();
        return report;
    }
};
