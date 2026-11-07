#pragma once
#include "../models/OrderInput.hpp"

class [[Service]] OrderService {
public:
    OrderInput accept([[Check]] const OrderInput& order) { return order; }
};
