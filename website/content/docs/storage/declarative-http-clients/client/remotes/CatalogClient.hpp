#pragma once
#include <string>
#include <vector>

class [[HTTPClient]] CatalogClient {
public:
    [[GetRequest("/api/products")]]
    Task<std::vector<std::string>> all();
};
