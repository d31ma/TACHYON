#pragma once
#include "../services/CatalogService.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
    [[Needs]] CatalogService& catalog;

public:
    [[GetRoute]]
    tachyon::json::JsonNode getCatalog() { return catalog.products(); }
};
