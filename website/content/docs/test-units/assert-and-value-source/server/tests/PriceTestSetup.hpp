#pragma once

class [[TestSetup]] PriceTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        return BehaviorAdapters();
    }
};
