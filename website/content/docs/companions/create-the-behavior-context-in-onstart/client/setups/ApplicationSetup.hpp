#pragma once

class [[Setup]] ApplicationSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        return BehaviorAdapters();
    }
};
