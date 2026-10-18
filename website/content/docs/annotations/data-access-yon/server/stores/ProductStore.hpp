#pragma once
#include <string>
#include <vector>
#include "../models/ProductEntity.hpp"

class [[Store(ProductEntity, {"fields":["id","status"],"defaultSize":20,"maxSize":100,"setup":"CatalogSetup"})]] ProductStore : public tachyon::Repository<ProductEntity, long long> {
public:
    std::vector<ProductEntity> findByStatus(const std::string& status);
    long long countByStatus(const std::string& status);

    [[Query({"operation":"find","where":[{"field":"status","operator":"ne","argument":0}],"orderBy":[{"field":"id","direction":"desc"}]})]]
    std::vector<ProductEntity> excluding(const std::string& status, [[PageDefaults]] tachyon::json::JsonNode page);
};
