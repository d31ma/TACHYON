#pragma once
#include "../errors/OrderNotFound.hpp"
#include "../models/Problem.hpp"

class [[RequestRoute("/orders")]] [[Public]] OrdersController {
public:
    [[GetRoute]]
    Problem getOrder() {
        throw OrderNotFound("order 42");
    }
};
