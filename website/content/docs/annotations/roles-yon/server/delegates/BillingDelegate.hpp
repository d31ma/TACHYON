#pragma once

class [[Delegate]] BillingDelegate {
    static constexpr int SURCHARGE = 2;

public:
    int settle(int subtotal) const { return subtotal + SURCHARGE; }
};
