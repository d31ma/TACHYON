#pragma once

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.http["catalog"] = [](const tachyon::http::HttpRequestMessage&) {
            tachyon::json::JsonArray products;
            products.add("anvil");
            products.add("rope");
            return tachyon::json::JsonNode(products);
        };
        return adapters;
    }
};
