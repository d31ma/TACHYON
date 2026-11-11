#pragma once
#include "../models/OrderInput.hpp"

class [[HTTPClient("/api")]] OrderClient {
public:
    [[PostRequest("/orders")]]
    Task<OrderInput> send([[RequestBody]] const OrderInput& order);
};
