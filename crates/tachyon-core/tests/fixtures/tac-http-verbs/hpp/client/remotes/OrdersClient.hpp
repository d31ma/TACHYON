#pragma once
#include <string>
#include "../models/Order.hpp"
#include "../models/SentRequest.hpp"

class [[HTTPClient({"path":"/api","adapter":"orders"})]] OrdersClient {
public:
    [[PutRequest("/orders/{id}")]]
    Task<SentRequest> replace([[PathVariable("id")]] std::string id, [[RequestBody]] const Order& order, [[RequestHeader("X-Tenant")]] std::string tenant);

    [[PatchRequest("/orders/{id}")]]
    Task<SentRequest> change([[PathVariable("id")]] std::string id, [[RequestBody]] const Order& change, [[RequestHeader("X-Tenant")]] std::string tenant);

    [[DeleteRequest("/orders/{id}")]]
    Task<SentRequest> remove([[PathVariable("id")]] std::string id, [[RequestHeader("X-Reason")]] std::string reason);
};
