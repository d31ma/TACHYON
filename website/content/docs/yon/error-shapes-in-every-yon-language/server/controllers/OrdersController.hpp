#pragma once
#include <stdexcept>
#include <string>
#include <vector>
#include "../models/OrderInput.hpp"

class [[RequestRoute("/orders")]] [[Allow(Admin)]] OrdersController {
    [[Needs]] OrderService& orders;

public:
    [[GetRoute]]
    std::vector<std::string> getOrders() {
        throw std::runtime_error("the order store is down");
    }

    [[PostRoute]]
    OrderInput postOrder([[FromBody]] const OrderInput& order) { return orders.accept(order); }
};
