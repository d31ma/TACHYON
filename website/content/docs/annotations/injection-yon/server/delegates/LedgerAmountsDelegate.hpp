#pragma once

class [[Delegate]] [[Lifetime("singleton")]] LedgerAmountsDelegate {
public:
    int subtotal() const { return 40; }
    int surcharge() const { return 2; }
};
