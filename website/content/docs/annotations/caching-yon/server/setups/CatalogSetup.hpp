#pragma once

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.caches["catalog"] = BehaviorMemory::cache();
        adapters.authorize = [](const BehaviorCacheCall& call, const BehaviorExecution&) {
            return call.owner == "CatalogService";
        };
        return adapters;
    }
};
