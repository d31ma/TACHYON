#pragma once

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        BehaviorAdapters adapters;
        adapters.http["catalog"] = [](const tachyon::http::HttpRequestMessage& request) {
            return tachyon::json::JsonNode::parse(R"({"items":["anvil"]})");
        };
        return adapters;
    }
};
