#pragma once

class [[Helper]] [[Lifetime("transient")]] LedgerHelper {
    [[Needs]] LedgerAmountsHelper& amounts;

public:
    int settle(int subtotal) const {
        int surcharge = amounts.surcharge();
        return subtotal + surcharge;
    }
};
