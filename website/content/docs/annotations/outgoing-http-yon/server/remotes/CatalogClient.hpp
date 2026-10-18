#pragma once

class [[HTTPClient]] CatalogClient {
public:
    [[GetRequest("/products")]]
    tachyon::json::JsonNode all([[RequestParam("page")]] int page) const;

    [[PostRequest("/products")]]
    tachyon::json::JsonNode add([[RequestBody]] const tachyon::json::JsonNode& product);
};
