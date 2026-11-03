#pragma once
#include "../models/OrderList.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrdersController {
    [[Needs]] OrderService& orders;

public:
    [[GetRoute]]
    OrderList getOrders() {
        return OrderList{orders.list()};
    }
};
