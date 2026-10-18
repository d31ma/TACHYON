#pragma once
#include <string>
#include "../models/PublicSettings.hpp"
#include "../remotes/ApplicationClient.hpp"

class [[Service]] SettingsService {
public:
    std::string api;
    [[Needs]] ApplicationClient& applicationClient;

    Task<void> load() {
        tachyon::json::JsonNode application = co_await applicationClient.load();
        api = Behaviors::values<PublicSettings>(application).api;
    }
};
