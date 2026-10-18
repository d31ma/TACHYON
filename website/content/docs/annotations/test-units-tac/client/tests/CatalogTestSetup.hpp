#pragma once

class [[TestSetup]] CatalogTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        return BehaviorAdapters();
    }
};
