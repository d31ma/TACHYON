#pragma once
#include "../errors/OrderNotFound.hpp"
#include "../models/Problem.hpp"

class [[ErrorHandler]] OrdersErrorHandler {
public:
    [[HandlesError]] [[ResponseStatus(404)]]
    Problem missing(const OrderNotFound& error) {
        return Problem{"order-not-found"};
    }
};
