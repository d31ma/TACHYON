#pragma once

class [[TestSetup]] GreetingTestSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        return BehaviorAdapters();
    }
};
