#pragma once

class [[Service]] LedgerService {
    [[Needs]] LedgerAmountsHelper& amounts;

public:
    int subtotal() const { return amounts.subtotal(); }
};
