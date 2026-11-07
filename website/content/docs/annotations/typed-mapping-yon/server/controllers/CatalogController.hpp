#pragma once
#include "../converters/ProductConverter.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
    [[Needs]] ProductConverter& converter;

public:
    [[GetRoute]]
    Summary getCatalog() {
        Detail detail{"detail", "detail-private"};
        return converter.map(Product{7, "Anvil", detail, {detail}, "product-private"});
    }
};
