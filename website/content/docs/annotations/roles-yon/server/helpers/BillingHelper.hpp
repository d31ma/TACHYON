#pragma once

class [[Helper]] BillingHelper {
    static constexpr int SURCHARGE = 2;

public:
    int settle(int subtotal) const { return subtotal + SURCHARGE; }
};
