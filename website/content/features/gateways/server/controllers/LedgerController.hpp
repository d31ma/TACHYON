#pragma once
#include "../models/Balance.hpp"
#include "../remotes/LedgerClient.hpp"

class [[RequestRoute("/ledger")]] [[Public]] LedgerController {
    [[Needs]] LedgerClient& ledgerClient;

public:
    [[GetRoute]]
    Balance getLedger() {
        return Balance{ledgerClient.balance("ACME-7"), ledgerClient.forecast(1)};
    }
};
