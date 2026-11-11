#pragma once
#include <string>
#include "../models/Order.hpp"

class [[TestSetup]] OrdersTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.http["orders"] = [](const tachyon::http::HttpRequestMessage& request) {
            if (request.requestUri == "/api/claims/a-7") {
                tachyon::json::JsonObject conflict;
                conflict["reason"] = "taken";
                conflict["holder"] = "south";
                throw tachyon::http::HttpRequestException("HTTP_STATUS", "private fake text", 409, tachyon::json::JsonNode(conflict));
            }
            std::string header;
            for (const auto& [name, value] : request.headers.all()) {
                header = value;
            }
            tachyon::json::JsonObject answer;
            answer["method"] = request.method.toString();
            answer["path"] = request.requestUri;
            answer["header"] = header;
            answer["item"] = request.content.has_value() ? request.content->readFromJson<Order>().item : std::string("none");
            return tachyon::json::JsonNode(answer);
        };
        return adapters;
    }
};
