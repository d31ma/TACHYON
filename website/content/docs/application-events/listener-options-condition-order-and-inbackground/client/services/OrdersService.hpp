#pragma once
#include <string>
#include <vector>
#include "../models/PlacedOrder.hpp"
#include "../models/SettledOrder.hpp"

class [[Service]] OrdersService {
public:
    ApplicationEventPublisher applicationEventPublisher;
    std::vector<std::string> log;
    int settled = 0;
    int audited = 0;

    void place(int total) {
        applicationEventPublisher.publishEvent(PlacedOrder{"paid", total}, false);
    }

    [[OnEvent({"id":"orders-log"})]] [[Order(-10)]]
    void record(const PlacedOrder& order) { log.push_back(order.status); }

    [[OnEvent({"condition":"value.status == 'paid' && value.total >= 10"})]]
    SettledOrder settleLargeOrder(const PlacedOrder& order) {
        settled += 1;
        return SettledOrder{order.total};
    }

    [[OnEvent]] [[InBackground]]
    void audit(const SettledOrder& value) { audited = value.total; }
};
