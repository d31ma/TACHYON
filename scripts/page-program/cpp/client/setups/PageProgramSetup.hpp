#pragma once
#include <memory>
#include <random>
#include <string>
#include "../models/ProgramState.hpp"

class [[Setup]] PageProgramSetup {
public:
    static BehaviorAdapters configure(const BehaviorExecution&) {
        auto state = std::make_shared<ProgramState>(ProgramState{identity(), 0});
        BehaviorAdapters adapters;
        adapters.http["pageprogram"] = [state](const tachyon::http::HttpRequestMessage& request) {
            if (request.method == tachyon::http::HttpMethod::Post && endsWith(request.requestUri, "/increment")) {
                state->count += 1;
            }
            return tachyon::json::JsonSerializer::serializeToNode(*state);
        };
        return adapters;
    }

private:
    static std::string identity() {
        std::random_device device;
        std::mt19937_64 random(device());
        const char* digits = "0123456789abcdef";
        std::string text;
        for (int index = 0; index < 32; index += 1) {
            text.push_back(digits[random() % 16]);
        }
        return text;
    }

    static bool endsWith(const std::string& text, const std::string& end) {
        return text.size() >= end.size() && text.compare(text.size() - end.size(), end.size(), end) == 0;
    }
};
