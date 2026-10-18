#pragma once
#include <string>
#include "../stores/ProductStore.hpp"

class [[Service]] ProductService {
public:
    std::string summary = "Not loaded.";
    [[Needs]] ProductStore& products;

    [[AllOrNothing({"adapter":"indexeddb"})]] void load() {
        products.save(ProductEntity{1, "active"});
        summary = std::to_string(products.countByStatus("active")) + " active products";
    }
};
