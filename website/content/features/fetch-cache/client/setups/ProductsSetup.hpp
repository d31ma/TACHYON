#pragma once

class [[Setup]] ProductsSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.http["products"] = [](const tachyon::http::HttpRequestMessage&) {
            tachyon::json::JsonArray items;
            items.add("anvil");
            items.add("hammer");
            tachyon::json::JsonObject answer;
            answer["items"] = items;
            return tachyon::json::JsonNode(answer);
        };
        return adapters;
    }
};
