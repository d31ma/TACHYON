#pragma once
#include <string>
#include <vector>
#include "../remotes/CatalogClient.hpp"

class [[Service]] CatalogService {
public:
    std::vector<std::string> products;
    std::string error;
    [[Needs]] CatalogClient& catalogClient;

    Task<void> load() {
        try {
            products = co_await catalogClient.all();
            error = "";
        } catch (const std::exception&) {
            error = "Products could not be loaded. Please retry.";
        }
    }
};
