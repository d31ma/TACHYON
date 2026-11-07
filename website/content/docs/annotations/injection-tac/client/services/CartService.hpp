#pragma once
#include "../delegates/CartDelegate.hpp"
#include "../delegates/CartPricesDelegate.hpp"

class [[Service, Lifetime({"value":"prototype"})]] CartService {
public:
    [[Needs]] CartPricesDelegate& cartPricesDelegate;
    [[Needs]] CartDelegate& cartDelegate;

    long long total() const {
        return cartDelegate.settle(cartPricesDelegate.subtotal());
    }
};
