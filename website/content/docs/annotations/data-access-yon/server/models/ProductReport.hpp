#pragma once
#include <optional>
#include <vector>
#include "ProductEntity.hpp"

struct ProductReport {
    std::vector<ProductEntity> active;
    long long count = 0;
    std::vector<ProductEntity> others;
    std::optional<ProductEntity> first;
};
