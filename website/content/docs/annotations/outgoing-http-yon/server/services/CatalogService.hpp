#pragma once
#include "../remotes/CatalogClient.hpp"

class [[Service]] CatalogService {
    [[Needs]] CatalogClient& catalog;

public:
    tachyon::json::JsonNode products() const { return catalog.all(1); }
};
