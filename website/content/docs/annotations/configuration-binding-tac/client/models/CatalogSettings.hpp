#pragma once
#include <string>

struct [[Settings({"prefix":"TAC_CATALOG_"})]] CatalogSettings {
    [[Min(1)]] [[Max(100)]] int pageSize = 20;
    bool cacheEnabled = false;
    std::string title = "Catalog";
};
