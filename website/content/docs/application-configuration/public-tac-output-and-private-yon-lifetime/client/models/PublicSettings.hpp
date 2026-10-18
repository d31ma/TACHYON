#pragma once
#include <string>

struct PublicSettings {
    [[Setting("${tac.api.url:/api}")]] std::string api;
};
