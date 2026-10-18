#pragma once
#include <optional>
#include <string>
#include "../models/Identity.hpp"

class [[IdentityProvider]] AppIdentityProvider {
public:
    std::optional<Identity> identify(const tachyon::http::HttpRequestMessage& request) {
        return find(request.headers.get("authorization").value_or(""));
    }

private:
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
