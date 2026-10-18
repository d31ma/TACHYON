#pragma once

class [[Helper]] [[Lifetime("singleton")]] LedgerAmountsHelper {
public:
    int subtotal() const { return 40; }
    int surcharge() const { return 2; }
};
