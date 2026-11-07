#pragma once
#include <string>

class [[Delegate]] TaxDelegate {
    [[Needs]] TaxService& taxService;

public:
    int rate(const std::string& region) const {
        return taxService.rate(region) * 2;
    }

    int total(int count, int extra) const {
        return taxService.total(count) + extra;
    }
};
