#pragma once
#include "../helpers/BillingHelper.hpp"

class [[Service]] BillingService {
    static constexpr int SUBTOTAL = 40;

    [[Needs]] BillingHelper& helper;

public:
    int total() const { return helper.settle(SUBTOTAL); }
};
