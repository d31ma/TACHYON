#pragma once
#include <chrono>
#include <cstdint>
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
            tachyon::json::JsonObject answer;
            answer["identity"] = state->identity;
            answer["count"] = state->count;
            return tachyon::json::JsonNode(answer);
        };
        return adapters;
    }

private:
    static std::string identity() {
        static std::uint64_t configured = 0;
        configured += 1;
        const auto now = std::chrono::steady_clock::now().time_since_epoch().count();
        std::mt19937_64 random(static_cast<std::uint64_t>(now) ^ (configured << 32));
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
