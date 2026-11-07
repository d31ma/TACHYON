#pragma once
#include "../models/CatalogTotal.hpp"

class [[RequestRoute("/api/catalog")]] [[Public]] CatalogController {
    [[Needs]] CatalogService& catalogService;

public:
    [[GetRoute]]
    CatalogTotal getTotal() {
        return CatalogTotal{catalogService.total()};
    }
};
