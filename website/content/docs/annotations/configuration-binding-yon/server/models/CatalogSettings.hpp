#pragma once

struct [[Settings({"prefix":"YON_CATALOG_"})]] CatalogSettings {
    int pageSize = 20;
    bool cacheEnabled = false;
};
