#pragma once

class [[Delegate]] TaxDelegate {
    [[Needs]] TaxRateDelegate& rates;

public:
    int addTax(int subtotal) const {
        int tax = rates.rate();
        return subtotal + tax;
    }
};
