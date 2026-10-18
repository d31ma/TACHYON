#pragma once
#include "../models/CatalogSettings.hpp"
#include "../models/Endpoint.hpp"
#include "../models/Settings.hpp"
#include "../setups/CatalogSetup.hpp"

class [[RequestRoute("/settings")]] [[Public]] SettingsController {
public:
    [[GetRoute]]
    Settings getSettings() {
        tachyon::json::JsonNode application = tachyon::yon::YonApplication::snapshot();
        CatalogSettings catalog = Behaviors::properties<CatalogSettings>(application);
        Endpoint endpoint = Behaviors::values<Endpoint>(application);
        BehaviorOptions options;
        options.partition = "public-catalog";
        options.profiles = {endpoint.profile};
        BehaviorContext context = Behaviors::create<CatalogSetup>(options.withApplication(application));
        context.dispose();
        return Settings{endpoint.profile, catalog.pageSize};
    }
};
