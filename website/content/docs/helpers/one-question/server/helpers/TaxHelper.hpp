#pragma once

class [[Helper]] TaxHelper {
    [[Needs]] TaxRateHelper& rates;

public:
    int addTax(int subtotal) const {
        int tax = rates.rate();
        return subtotal + tax;
    }
};
