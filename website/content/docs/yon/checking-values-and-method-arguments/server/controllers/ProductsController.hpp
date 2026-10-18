#pragma once
#include "../models/Product.hpp"
#include "../models/ProductInput.hpp"

class [[RequestRoute("/products")]] [[Public]] ProductsController {
    [[Needs]] ProductService& productService;

public:
    [[PostRoute]]
    Product postProduct([[FromBody]] const Product& product) {
        productService.accept(ProductInput{product.name, product.level});
        return product;
    }
};
