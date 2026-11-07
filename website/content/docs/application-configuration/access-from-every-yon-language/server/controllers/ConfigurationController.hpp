#pragma once
#include "../models/CatalogSettings.hpp"
#include "../models/Configuration.hpp"
#include "../models/Endpoint.hpp"

class [[RequestRoute("/configuration")]] [[Public]] ConfigurationController {
public:
    [[GetRoute]]
    Configuration getConfiguration() {
        tachyon::json::JsonNode application = tachyon::yon::YonApplication::snapshot();
        CatalogSettings settings = Behaviors::properties<CatalogSettings>(application);
        Endpoint endpoint = Behaviors::values<Endpoint>(application);
        return Configuration{settings.pageSize, endpoint.port};
    }
};
