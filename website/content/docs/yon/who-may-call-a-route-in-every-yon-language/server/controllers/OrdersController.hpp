#pragma once
#include <string>
#include <vector>

class [[RequestRoute("/orders")]] [[SignedIn]] OrdersController {
public:
    [[GetRoute]] [[Public]]
    std::vector<std::string> getOrders() {
        return {"lamp", "desk"};
    }

    [[PostRoute]]
    std::vector<std::string> postOrder() {
        return {"chair"};
    }

    [[DeleteRoute]] [[Allow(Admin)]]
    std::vector<std::string> deleteOrders() {
        return {};
    }
};
