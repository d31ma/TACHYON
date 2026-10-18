#pragma once
#include "../setups/ApplicationSetup.hpp"

class [[Service]] CatalogService {
public:
    BehaviorContext behaviors;
    long long offset = 0;

    tachyon::json::JsonNode page([[PageDefaults({"defaultSize":20,"maxSize":100,"sort":["id"]})]] tachyon::json::JsonNode input) {
        return input["offset"];
    }

    [[OnStart]] void initialize() {
        BehaviorOptions options;
        options.partition = "public-catalog";
        behaviors = Behaviors::create<ApplicationSetup>(options);
        offset = static_cast<long long>(page(tachyon::json::JsonNode::parse(R"({"page":1})")).getValue<double>());
    }
};
