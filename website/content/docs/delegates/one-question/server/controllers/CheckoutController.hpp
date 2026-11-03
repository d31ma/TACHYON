#pragma once
#include "../models/Receipt.hpp"

class [[RequestRoute("/checkout")]] [[Public]] CheckoutController {
    [[Needs]] CheckoutService& service;

public:
    [[GetRoute]]
    Receipt getCheckout() {
        int total = service.total();
        return Receipt{total};
    }
};
