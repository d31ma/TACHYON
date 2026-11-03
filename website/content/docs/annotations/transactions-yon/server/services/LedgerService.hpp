#pragma once
#include <string>
#include "../models/LedgerReport.hpp"

class [[Service]] LedgerService {
    BehaviorContext behaviors;
    [[Needs]] EntryStore& entries;

public:
    [[AllOrNothing({"adapter":"ledger"})]]
    LedgerReport settle(std::string status) {
        LedgerReport report;
        report.open = static_cast<int>(entries.findByStatus(status).size());
        report.total = entries.countByStatus(status);
        return report;
    }

private:
    [[OnStart]]
    void start() {
        BehaviorOptions options;
        options.partition = "ledger";
        behaviors = Behaviors::create<LedgerSetup>(options);
    }

    [[OnStop]]
    void stop() { behaviors.dispose(); }
};
