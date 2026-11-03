#pragma once
#include "../models/Lifecycle.hpp"

class [[RequestRoute("/lifecycle")]] [[Public]] LifecycleController {
    [[Needs]] AuditService& audit;
    [[Needs]] CheckoutService& checkout;

public:
    [[GetRoute]]
    Lifecycle getLifecycle() {
        return Lifecycle{audit.log, audit.settled};
    }
};
