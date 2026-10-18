#pragma once
#include <string>
#include <vector>

class [[Service]] AuditService {
public:
    std::vector<std::string> log;
    long long settled = 0;

private:
    [[OnEvent({"id":"audit-log"})]] [[Order(-10)]]
    void record(const PlacedOrder& order) { log.push_back(order.status); }

    [[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]] [[Order(10)]]
    SettledOrder settle(const PlacedOrder& order) { return SettledOrder{order.total}; }

    [[OnEvent]]
    void saveSettled(const SettledOrder& order) { settled = order.total; }
};
