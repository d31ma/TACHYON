#pragma once

class [[Setup, WhenProfile("public"), WhenSetting({"name":"TAC_CATALOG_CACHE_ENABLED","havingValue":"true","matchIfMissing":false})]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        return BehaviorAdapters();
    }
};
