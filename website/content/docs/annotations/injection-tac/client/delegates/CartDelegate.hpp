#pragma once
#include "CartPricesDelegate.hpp"

class [[Delegate]] CartDelegate {
public:
    [[Needs]] CartPricesDelegate& cartPricesDelegate;

    long long settle(long long subtotal) const {
        return subtotal + cartPricesDelegate.surcharge();
    }

    long long count() const { return cartPricesDelegate.count(); }
};
