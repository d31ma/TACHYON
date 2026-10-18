#pragma once
#include "../models/OrderReport.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrdersController {
    [[Needs]] OrdersService& orders;

public:
    [[GetRoute]]
    OrderReport getOrders() {
        OrderReport answer;
        answer.submitted = orders.submit("A-1");
        answer.report = orders.report("A-1");
        return answer;
    }
};
