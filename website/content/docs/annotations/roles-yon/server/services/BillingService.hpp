#pragma once
#include "../delegates/BillingDelegate.hpp"

class [[Service]] BillingService {
    static constexpr int SUBTOTAL = 40;

    [[Needs]] BillingDelegate& helper;

public:
    int total() const { return helper.settle(SUBTOTAL); }
};
