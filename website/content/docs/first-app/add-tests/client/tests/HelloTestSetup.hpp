#pragma once
#include "../models/Greeting.hpp"

class [[TestSetup]] HelloTestSetup {
public:
    static BehaviorAdapters configure() {
        BehaviorAdapters adapters;
        adapters.http["greeting"] = [](const tachyon::http::HttpRequestMessage&) {
            return tachyon::json::JsonSerializer::serializeToNode(Greeting{"Hi"});
        };
        return adapters;
    }
};
