#pragma once
#include <vector>
#include "../models/Product.hpp"

class [[Service]] InventoryService {
public:
    std::vector<Product> products{{"Anvil"}, {"Rope"}};
};
