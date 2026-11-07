#pragma once
#include "../entities/BillingEntity.hpp"

class [[Store(BillingEntity, {"fields":["id"],"defaultSize":20,"maxSize":100})]] BillingStore : public tachyon::Repository<BillingEntity, long long> {
public:
    bool existsById(long long id);
};
