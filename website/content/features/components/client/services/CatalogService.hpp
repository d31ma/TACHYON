#pragma once
#include <vector>
#include "../models/Product.hpp"

class [[Service]] CatalogService {
public:
    std::vector<Product> products{
        {"anvil", "Anvil", "£42.00"},
        {"rope", "Rope", "£8.00"},
    };
};
