#pragma once
#include <string>
#include "../models/CatalogSettings.hpp"
#include "../models/PublicValues.hpp"
#include "../setups/CatalogSetup.hpp"

class [[Service]] SettingsService {
public:
    std::string title = "Catalog";
    int pageSize = 20;

    [[OnStart]] void start() {
        tachyon::json::JsonNode published = tachyon::json::JsonNode::parse(R"({"tac":{"catalog":{"title":"Catalog","page":{"size":20},"cache":{"enabled":true}}}})");
        title = Behaviors::values<PublicValues>(published).title;
        pageSize = Behaviors::properties<CatalogSettings>(published).pageSize;
        BehaviorOptions options;
        options.partition = "public-catalog";
        options.profiles = {"public"};
        BehaviorContext context = Behaviors::create<CatalogSetup>(options.withApplication(published));
        context.dispose();
    }
};
