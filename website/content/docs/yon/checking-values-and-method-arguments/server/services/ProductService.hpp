#pragma once
#include "../models/ProductInput.hpp"

class [[Service]] ProductService {
public:
    ProductInput accept([[Check]] const ProductInput& input) { return input; }
};
