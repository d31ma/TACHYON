#pragma once

class [[Setup]] BillingSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) { return BehaviorAdapters(); }
};
