#pragma once

class [[TestSetup]] CatalogTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        return BehaviorAdapters();
    }
};
