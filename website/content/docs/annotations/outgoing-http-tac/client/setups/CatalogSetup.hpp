#pragma once

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.http["catalog"] = [](const tachyon::http::HttpRequestMessage&) {
            tachyon::json::JsonArray items;
            items.add("anvil");
            tachyon::json::JsonObject answer;
            answer["items"] = items;
            return tachyon::json::JsonNode(answer);
        };
        return adapters;
    }
};
