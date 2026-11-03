#pragma once
#include <string>

class [[Service, CacheSettings({"name":"catalog","ttlMs":30000,"maxBytes":65536,"maxEntries":256})]] CatalogService {
    BehaviorContext behaviors;
    int reads = 0;

public:
    [[Cached]]
    tachyon::json::JsonNode find(std::string sku) {
        reads += 1;
        return entry(sku, reads);
    }

    [[UpdateCache({"method":"find"})]]
    tachyon::json::JsonNode write(std::string sku, int count) {
        return entry(sku, count);
    }

    [[ClearCache({"method":"find"})]]
    tachyon::json::JsonNode invalidate(std::string sku) {
        return entry(sku, reads);
    }

private:
    [[OnStart]]
    void open() {
        BehaviorOptions options;
        options.partition = "public-catalog";
        behaviors = Behaviors::create<CatalogSetup>(options);
    }

    [[OnStop]]
    void close() { behaviors.dispose(); }

    static tachyon::json::JsonNode entry(const std::string& sku, int reads) {
        tachyon::json::JsonObject entry;
        entry["sku"] = sku;
        entry["reads"] = reads;
        return entry;
    }
};
