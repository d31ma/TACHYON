#pragma once
#include "../models/ProductReport.hpp"

class [[RequestRoute("/products")]] [[Public]] ProductController {
    [[Needs]] ProductStore& products;

public:
    [[GetRoute]]
    ProductReport getProduct() {
        ProductReport report;
        report.active = products.findByStatus("active");
        report.count = products.countByStatus("active");
        report.others = products.excluding("active", tachyon::json::JsonNode::parse(R"({"size":5})"));
        report.first = products.findById(1);
        return report;
    }
};
