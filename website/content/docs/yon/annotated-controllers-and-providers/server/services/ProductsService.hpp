#pragma once
#include <cstdint>
#include <vector>

class [[Service]] [[Lifetime("transient")]] ProductsService {
public:
    std::vector<Product> list() const { return {find(1)}; }

    Product find(std::int64_t id) const {
        Product product;
        product.id = id;
        product.name = "Anvil";
        return product;
    }
};
