#pragma once

class [[Service]] CheckoutService {
    [[Needs]] TaxDelegate& helper;

public:
    int total() const {
        int amount = subtotal();
        return helper.addTax(amount);
    }

private:
    int subtotal() const { return 40; }
};
