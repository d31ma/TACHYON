#pragma once

class [[Service]] LedgerService {
    [[Needs]] LedgerAmountsDelegate& amounts;

public:
    int subtotal() const { return amounts.subtotal(); }
};
