#pragma once
#include "../converters/ProductConverter.hpp"
#include "../models/CatalogSummary.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
    [[Needs]] ProductConverter& converter;

public:
    [[GetRoute]]
    CatalogSummary getCatalog() {
        tachyon::json::JsonNode product = tachyon::json::JsonNode::parse(R"({"detail":{"id":7},"name":"Anvil","extra":"dropped"})");
        return CatalogSummary{converter.toSummary(product)};
    }
};
