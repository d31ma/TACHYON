#pragma once
#include <optional>
#include <stdexcept>
#include <string>
#include "../models/Identity.hpp"

class [[IdentityProvider]] AppIdentityProvider {
public:
    std::optional<Identity> identify(const tachyon::http::HttpRequestMessage& request) {
        std::string token = request.headers.get("authorization").value_or("");
        refuseBroken(token);
        return find(token);
    }

private:
    static void refuseBroken(const std::string& token) {
        if (token == "broken-token") {
            throw std::runtime_error("the session store is down");
        }
    }

    static std::optional<Identity> find(const std::string& token) {
        if (token == "ada-token") {
            return Identity{"ada", {"Admin"}};
        }
        if (token == "bob-token") {
            return Identity{"bob", {}};
        }
        return std::nullopt;
    }
};
