#pragma once
#include <string>
#include "../models/PlacedOrder.hpp"

class [[Service]] AuditService {
public:
    int heard = 0;
    std::string lastStatus;

    [[OnEvent]]
    void record(const PlacedOrder& order) {
        heard += order.total;
        lastStatus = order.status;
    }
};
