#pragma once
#include "../models/Catalog.hpp"

class [[HTTPClient]] CatalogClient {
public:
    [[GetRequest("/api/products")]]
    Task<Catalog> all();
};
