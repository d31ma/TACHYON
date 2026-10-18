#pragma once

class [[Setup]] BillingSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution& execution) {
        return BehaviorAdapters();
    }
};
