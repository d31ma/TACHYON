#pragma once
#include <vector>
#include "../models/Product.hpp"

class [[Service]] StockService {
public:
    std::vector<Product> products;
    bool restocking = true;
};
