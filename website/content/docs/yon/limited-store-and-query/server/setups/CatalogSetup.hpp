#pragma once
#include <string>

class [[Setup]] CatalogSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.entities["catalog"] = [](const tachyon::json::JsonNode& command, const BehaviorExecution&) -> tachyon::json::JsonNode {
            tachyon::json::JsonNode rows = tachyon::json::JsonNode::parse(R"([{"id":1,"status":"active"},{"id":2,"status":"retired"}])");
            const tachyon::json::JsonNode& condition = command["query"]["where"][std::size_t{0}];
            bool unequal = condition["operator"].getValue<std::string>() == "ne";
            tachyon::json::JsonArray matching;
            for (const tachyon::json::JsonNode& row : rows.items()) {
                bool equal = row[condition["field"].getValue<std::string>()] == condition["value"];
                if (equal != unequal) {
                    matching.add(row);
                }
            }
            if (command["query"]["kind"].getValue<std::string>() == "count") {
                return tachyon::json::JsonNode(static_cast<long long>(matching.count()));
            }
            return matching;
        };
        adapters.authorizeEntity = [](const tachyon::json::JsonNode& command, const BehaviorExecution&) {
            return command["entity"].getValue<std::string>() == "ProductEntity";
        };
        return adapters;
    }
};
