#pragma once
#include "../delegates/CartDelegate.hpp"

class [[Service]] AuditService {
public:
    [[Needs, Named("CartDelegate")]] CartDelegate& ledger;

    long long lines() const { return ledger.count(); }
};
