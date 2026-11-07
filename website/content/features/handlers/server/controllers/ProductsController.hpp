#pragma once
#include <vector>
#include "../models/Product.hpp"

class [[RequestRoute("/products")]] [[Public]] ProductsController {
public:
    [[GetRoute]]
    std::vector<Product> getProducts() {
        return {Product{"anvil"}};
    }

    [[PostRoute]] [[ResponseStatus(201)]]
    Product postProduct([[FromBody]] Product product) {
        return product;
    }
};
