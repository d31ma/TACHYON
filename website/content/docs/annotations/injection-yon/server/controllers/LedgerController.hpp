#pragma once
#include "../models/Ledger.hpp"

class [[RequestRoute("/ledger")]] [[Public]] LedgerController {
    [[Needs]] [[Named("LedgerService")]] LedgerService& service;
    [[Needs]] [[Named("LedgerHelper")]] LedgerHelper& settlement;

public:
    [[GetRoute]]
    Ledger getLedger() {
        int subtotal = service.subtotal();
        int total = settlement.settle(subtotal);
        return Ledger{total};
    }
};
