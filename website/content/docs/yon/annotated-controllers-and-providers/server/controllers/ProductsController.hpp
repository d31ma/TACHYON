#pragma once
#include <cstdint>
#include <vector>
#include "../models/Product.hpp"

class [[RequestRoute("/products")]] [[Public]] ProductsController {
public:
    [[GetRoute]]
    std::vector<Product> getProducts() {
        return {find(1)};
    }

    [[GetRoute("/{id}")]]
    Product getProduct([[FromPath]] std::int64_t id) {
        return find(id);
    }

private:
    Product find(std::int64_t id) {
        Product product;
        product.id = id;
        product.name = "Anvil";
        return product;
    }
};
