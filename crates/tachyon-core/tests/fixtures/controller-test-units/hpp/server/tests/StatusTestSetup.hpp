#pragma once

class [[TestSetup]] StatusTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        return BehaviorAdapters();
    }
};
