#pragma once
#include "../remotes/CatalogClient.hpp"

class [[Service]] CatalogService {
public:
    int count = 0;
    [[Needs]] CatalogClient& catalogClient;

    Task<void> load() {
        Catalog catalog = co_await catalogClient.all();
        count = static_cast<int>(catalog.items.size());
    }
};
