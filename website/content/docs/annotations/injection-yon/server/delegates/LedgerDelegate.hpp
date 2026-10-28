#pragma once

class [[Delegate]] [[Lifetime("transient")]] LedgerDelegate {
    [[Needs]] LedgerAmountsDelegate& amounts;

public:
    int settle(int subtotal) const {
        int surcharge = amounts.surcharge();
        return subtotal + surcharge;
    }
};
