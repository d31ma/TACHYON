#pragma once
#include <string>

class [[Service]] OrdersService {
public:
    int attempts = 0;

    [[Retry({"maxRetries":3,"delayMs":10,"multiplier":2,"maxDelayMs":200,"jitterMs":5})]] tachyon::json::JsonNode submit(std::string id) {
        attempts += 1;
        if (attempts < 3) {
            throw tachyon::TacCodedError("the checkout API is unavailable", "adapter");
        }
        return reportOf(id);
    }

    [[MaxAtOnce({"limit":2,"policy":"block"})]] tachyon::json::JsonNode report(std::string id) {
        return reportOf(id);
    }

private:
    tachyon::json::JsonNode reportOf(const std::string& id) const {
        tachyon::json::JsonObject report;
        report["id"] = id;
        report["attempts"] = attempts;
        return report;
    }
};
