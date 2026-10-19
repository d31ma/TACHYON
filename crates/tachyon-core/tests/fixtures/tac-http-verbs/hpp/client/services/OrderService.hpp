#pragma once
#include "../remotes/OrdersClient.hpp"

class [[Service]] OrderService {
public:
    [[Needs]] OrdersClient& ordersClient;

    Task<SentRequest> replace() {
        co_return co_await ordersClient.replace("a-7", Order{"hammer"}, "north");
    }

    Task<SentRequest> change() {
        co_return co_await ordersClient.change("a-7", Order{"nails"}, "south");
    }

    Task<SentRequest> remove() {
        co_return co_await ordersClient.remove("a-7", "duplicate");
    }
};
