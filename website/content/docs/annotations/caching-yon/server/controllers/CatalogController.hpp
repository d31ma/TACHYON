#pragma once
#include "../models/CacheReport.hpp"

class [[RequestRoute("/catalog")]] [[Public]] CatalogController {
    [[Needs]] CatalogService& catalog;

public:
    [[GetRoute]]
    CacheReport getCatalog() {
        CacheReport report;
        report.first = catalog.find("anvil");
        report.repeat = catalog.find("anvil");
        catalog.write("anvil", 99);
        report.written = catalog.find("anvil");
        catalog.invalidate("anvil");
        report.refreshed = catalog.find("anvil");
        return report;
    }
};
