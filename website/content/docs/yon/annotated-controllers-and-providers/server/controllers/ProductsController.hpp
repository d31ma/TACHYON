#pragma once
#include <cstdint>
#include <vector>
#include "../models/Product.hpp"

class [[RequestRoute("/products")]] [[Public]] ProductsController {
    [[Needs]] [[Named("ProductsService")]] ProductsService& catalog;

public:
    [[GetRoute]]
    std::vector<Product> getProducts() { return catalog.list(); }

    [[GetRoute("/{id}")]]
    Product getProduct([[FromPath]] std::int64_t id) { return catalog.find(id); }
};
