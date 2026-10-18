#pragma once
#include <string>

struct Endpoint {
    [[Setting("${yon.active.profile:development}")]] std::string profile;
};
