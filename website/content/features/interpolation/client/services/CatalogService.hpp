#pragma once
#include <string>
#include "../models/Product.hpp"

class [[Service]] CatalogService {
public:
    std::string title = "Catalog";
    int count = 6;
    Product product{"Anvil", "£42.00"};
};
