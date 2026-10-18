#pragma once
#include <string>

class [[Service, CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]] CatalogService {
public:
    std::string sku = "anvil";
    int reads = 0;

    [[Cached]] tachyon::json::JsonNode find(std::string sku) {
        reads += 1;
        return entry(sku, reads);
    }

    [[UpdateCache({"method":"find"})]] tachyon::json::JsonNode write(std::string sku, int reads) {
        return entry(sku, reads);
    }

    [[ClearCache({"method":"find"})]] tachyon::json::JsonNode invalidate(std::string sku) {
        return entry(sku, reads);
    }

private:
    static tachyon::json::JsonNode entry(const std::string& sku, int reads) {
        tachyon::json::JsonObject entry;
        entry["sku"] = sku;
        entry["reads"] = reads;
        return entry;
    }
};
