#pragma once
#include "../remotes/ProductsClient.hpp"

class [[Service]] CatalogService {
public:
    [[Needs]] ProductsClient& productsClient;
    int count = 0;

    Task<void> load() {
        tachyon::json::JsonNode answer = co_await productsClient.list();
        count = static_cast<int>(answer["items"].count());
    }

    Task<void> add() {
        tachyon::json::JsonObject product;
        product["name"] = "anvil";
        co_await productsClient.save(product);
    }
};
