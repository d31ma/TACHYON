#pragma once
#include "../models/OrderList.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrdersController {
public:
    [[GetRoute]]
    OrderList getOrders() {
        OrderList list;
        list.orders.push_back("anvil");
        return list;
    }
};
