#pragma once

class [[Schedule({"fixedDelaySeconds":300})]] Handler {
    [[Needs]] LedgerService& ledger;

public:
    int POST() { return ledger.total(); }
};
