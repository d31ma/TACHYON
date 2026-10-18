#pragma once
#include "../models/OrderInput.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrdersController {
    [[Needs]] OrderService& orders;

public:
    [[PostRoute]]
    OrderInput postOrders([[FromBody]] const OrderInput& order) { return orders.accept(order); }
};
