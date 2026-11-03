#pragma once
#include "../models/PlacedOrder.hpp"
#include "../models/SettledOrder.hpp"

class [[Service]] AuditService {
public:
    int settled = 0;

private:
    int records = 0;
    int audited = 0;

    [[OnEvent({"id":"audit-log","replay":true})]] [[Order(-10)]]
    void record(const PlacedOrder& order) { records += 1; }

    [[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]] [[Order(10)]]
    SettledOrder settle(const PlacedOrder& order) { return SettledOrder{order.total}; }

    [[OnEvent]]
    void saveSettled(const SettledOrder& order) { settled = order.total; }

    [[OnEvent]] [[InBackground]]
    void audit(const SettledOrder& order) { audited = order.total; }
};
