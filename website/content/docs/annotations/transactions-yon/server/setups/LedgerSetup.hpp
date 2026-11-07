#pragma once
#include <string>
#include <vector>

class [[Setup]] LedgerSetup {
public:
    static std::vector<std::string>& commands() {
        static std::vector<std::string> logged;
        return logged;
    }

    static BehaviorAdapters configure(const BehaviorExecution&) {
        BehaviorAdapters adapters;
        adapters.entities["ledger"] = [](const tachyon::json::JsonNode& command, const BehaviorExecution&) -> tachyon::json::JsonNode {
            std::string transaction = command.containsKey("transaction") ? command["transaction"].getValue<std::string>() : "";
            if (command["operation"].getValue<std::string>() == "transaction") {
                std::string action = command["action"].getValue<std::string>();
                commands().push_back(transaction.empty() ? action : action + ":" + transaction);
                return tachyon::json::JsonNode("t1");
            }
            std::string kind = command["query"]["kind"].getValue<std::string>();
            commands().push_back(kind + "@" + (transaction.empty() ? "none" : transaction));
            if (kind == "count") {
                return tachyon::json::JsonNode(1);
            }
            return tachyon::json::JsonNode::parse(R"([{"id":1,"status":"open"}])");
        };
        adapters.authorizeEntity = [](const tachyon::json::JsonNode&, const BehaviorExecution&) { return true; };
        return adapters;
    }
};
