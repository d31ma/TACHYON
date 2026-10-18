#pragma once

class [[Setup]] OverrideSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.http["override"] = [](const tachyon::http::HttpRequestMessage&) {
            tachyon::json::JsonObject answer;
            answer["message"] = "Custom adapter wins";
            return tachyon::json::JsonNode(answer);
        };
        return adapters;
    }
};
